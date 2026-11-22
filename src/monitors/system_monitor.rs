use std::collections::HashMap;
use std::time::Instant;
use std::fs;

use crate::sys::procfs::{self, CpuInfo, CpuTicks, DiskStatsData, MemInfo};
use crate::sys::sysfs::{self, HwmonScanner};
use crate::types::*;
use crate::utils::*;

pub struct SystemMonitor {
    hwmon_scanner: HwmonScanner,
    cached_sensors: Vec<SensorInfo>,
    users_cache: UsersCache,
    prev_proc_ticks: HashMap<Pid, u64>,
    prev_proc_io: HashMap<Pid, (u64, u64)>,
    last_proc_cpu: HashMap<Pid, f32>,
    prev_net_usage: HashMap<String, NetworkStats>,
    prev_disk_stats: HashMap<String, DiskStatsData>,
    prev_global_cpu: CpuTicks,
    curr_global_cpu: CpuTicks,
    prev_core_cpus: Vec<CpuTicks>,
    curr_core_cpus: Vec<CpuTicks>,
    mem_info: MemInfo,
    cpu_info: CpuInfo,
    boot_time: u64,
    last_proc_update: Instant,
    last_disk_update: Instant,
    last_net_update: Instant,
    self_pid: u32,
    memory_details_cache: Option<(String, String, String, String)>,
}

fn get_cpu_caches() -> (String, String, String) {
    let mut l1 = String::new();
    let mut l2 = String::new();
    let mut l3 = String::new();

    let mut l1i = None;
    let mut l1d = None;

    for i in 0..6 {
        let dir = format!("/sys/devices/system/cpu/cpu0/cache/index{}", i);
        let level_path = format!("{}/level", dir);
        let type_path = format!("{}/type", dir);
        let size_path = format!("{}/size", dir);

        if let (Ok(lvl), Ok(typ), Ok(sz)) = (
            fs::read_to_string(&level_path),
            fs::read_to_string(&type_path),
            fs::read_to_string(&size_path),
        ) {
            let level = lvl.trim();
            let cache_type = typ.trim().to_lowercase();
            let size = sz.trim().to_string();

            if level == "1" {
                if cache_type == "instruction" {
                    l1i = Some(size);
                } else if cache_type == "data" {
                    l1d = Some(size);
                } else {
                    l1 = size;
                }
            } else if level == "2" {
                l2 = size;
            } else if level == "3" {
                l3 = size;
            }
        }
    }

    if l1.is_empty() {
        match (l1d, l1i) {
            (Some(d), Some(i)) => l1 = format!("{} d / {} i", d, i),
            (Some(d), None) => l1 = format!("{} d", d),
            (None, Some(i)) => l1 = format!("{} i", i),
            _ => l1 = "N/A".to_string(),
        }
    }
    if l2.is_empty() {
        l2 = "N/A".to_string();
    }
    if l3.is_empty() {
        l3 = "N/A".to_string();
    }

    (l1, l2, l3)
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut hwmon_scanner = HwmonScanner::new();
        let cached_sensors = hwmon_scanner.scan();
        let (global_cpu, core_cpus) = procfs::read_cpu_ticks();
        let mem_info = procfs::read_meminfo();
        let cpu_info = procfs::read_cpu_info();
        let (_, boot_time) = procfs::read_uptime_and_boot_time();
        let now = Instant::now();
        
        Self {
            hwmon_scanner,
            cached_sensors,
            users_cache: UsersCache::new(),
            prev_proc_ticks: HashMap::new(),
            prev_proc_io: HashMap::new(),
            last_proc_cpu: HashMap::new(),
            prev_net_usage: HashMap::new(),
            prev_disk_stats: HashMap::new(),
            prev_global_cpu: global_cpu,
            curr_global_cpu: global_cpu,
            prev_core_cpus: core_cpus.clone(),
            curr_core_cpus: core_cpus,
            mem_info,
            cpu_info,
            boot_time,
            last_proc_update: now,
            last_disk_update: now,
            last_net_update: now,
            self_pid: std::process::id(),
            memory_details_cache: None,
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        let (uptime, boot_time) = procfs::read_uptime_and_boot_time();
        let load = procfs::read_load_average();
        let (l1, l2, l3) = get_cpu_caches();

        let mut info = vec![
            ("OS".into(), procfs::read_os_name()),
            ("Kernel".into(), procfs::read_kernel_version()),
            ("Hostname".into(), procfs::read_hostname()),
            ("CPU".into(), self.cpu_info.brand.clone()),
            ("Cores".into(), format!("{} Physical / {} Logical",
                self.cpu_info.physical_cores,
                self.cpu_info.logical_cores)),
            ("Total Memory".into(), format_size(self.mem_info.total)),
            ("Vendor".into(), self.cpu_info.vendor.clone()),
            ("Family".into(), self.cpu_info.family.clone()),
            ("L1 Cache".into(), l1),
            ("L2 Cache".into(), l2),
            ("L3 Cache".into(), if l3 != "N/A" { l3 } else { self.cpu_info.cache_size.clone() }),
            ("BogoMIPS".into(), self.cpu_info.bogomips.clone()),
            ("Virtualization".into(), self.cpu_info.virtualization.clone()),
        ];

        info.extend(vec![
            ("Boot Time".into(), {
                if boot_time > 0 {
                    format_timestamp(boot_time as i64, "%Y-%m-%d %H:%M:%S")
                } else {
                    "Unknown".to_string()
                }
            }),
            ("Uptime".into(), {
                if uptime > 0 {
                    format_duration(uptime)
                } else {
                    "Unknown".to_string()
                }
            }),
            ("Load Average".into(), format!("{:.2}, {:.2}, {:.2}", load.0, load.1, load.2)),
        ]);
        info
    }

    pub fn get_total_memory(&self) -> u64 {
        self.mem_info.total
    }
    
    pub fn get_memory_details(&self) -> (String, String, String, String) {
        let mut mem_type = "N/A".to_string();
        let mut mem_gen = "N/A".to_string(); 
        let mut mem_speed = "N/A".to_string();
        let mut mem_temp = "N/A".to_string();

        if let Ok(chassis) = fs::read_to_string("/sys/class/dmi/id/chassis_type") {
            if let Ok(c) = chassis.trim().parse::<u32>() {
                mem_type = if [8, 9, 10, 11, 14, 30, 31, 32].contains(&c) {
                    "SODIMM".to_string()
                } else {
                    "DIMM".to_string()
                };
            }
        }

        if let Some(dmi) = sysfs::read_dmi_memory_info() {
            if let Some(ff) = dmi.form_factor {
                mem_type = ff;
            }
            if let Some(mt) = dmi.mem_type {
                mem_gen = mt;
            }
            if let Some(spd) = dmi.speed {
                mem_speed = spd;
            }
        }

        if mem_gen == "N/A" || mem_speed == "N/A" {
            if let Ok(output) = std::process::Command::new("dmidecode")
                .args(["-t", "17"])
                .output()
            {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("Type:") && mem_gen == "N/A" {
                            let t = line.split(':').next_back().unwrap_or("").trim();
                            if !t.is_empty() && !["Unknown", "Other", "<OUT OF SPEC>"].contains(&t) {
                                mem_gen = t.to_string();
                            }
                        } else if line.starts_with("Speed:") && mem_speed == "N/A" {
                            let s = line.split(':').next_back().unwrap_or("").trim();
                            if !s.is_empty() && !["Unknown", "Unknown Speed", "0 MT/s", "0 MHz"].contains(&s) {
                                mem_speed = s.to_string();
                            }
                        } else if line.starts_with("Form Factor:") && mem_type == "N/A" {
                            let f = line.split(':').next_back().unwrap_or("").trim();
                            if !f.is_empty() && f != "Unknown" {
                                mem_type = f.to_string();
                            }
                        }
                    }
                }
            }
        }

        if mem_gen == "N/A" || mem_gen == "Unknown" {
            if let Ok(entries) = fs::read_dir("/sys/devices/system/edac/mc") {
                for entry in entries.flatten() {
                    let mc_path = entry.path();
                    if let Ok(dimm_entries) = fs::read_dir(&mc_path) {
                        for dimm_entry in dimm_entries.flatten() {
                            let fname = dimm_entry.file_name().to_string_lossy().into_owned();
                            if fname.starts_with("dimm") || fname.starts_with("rank") {
                                if let Ok(dtype) = fs::read_to_string(dimm_entry.path().join("dimm_dev_type")) {
                                    let dt = dtype.trim();
                                    if !dt.is_empty() && dt != "Unknown" {
                                        mem_gen = dt.to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if mem_gen != "N/A" && mem_gen != "Unknown" { break; }
                }
            }
        }

        if let Some(t) = sysfs::find_memory_temp(&self.cached_sensors) {
            mem_temp = format!("{:.1}°C", t);
        }

        (mem_type, mem_gen, mem_speed, mem_temp)
    }
    
    pub fn refresh_core_metrics(&mut self) {
        let (global_cpu, core_cpus) = procfs::read_cpu_ticks();
        self.prev_global_cpu = self.curr_global_cpu;
        self.curr_global_cpu = global_cpu;
        self.prev_core_cpus = std::mem::replace(&mut self.curr_core_cpus, core_cpus);
        self.mem_info = procfs::read_meminfo();
        self.cached_sensors = self.hwmon_scanner.scan();
    }

    pub fn update_processes(&mut self, show_system: bool, filter: &str) -> Vec<ProcessInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_proc_update).as_secs_f64().max(0.1);
        self.last_proc_update = now;

        let scanned = procfs::scan_processes();
        let total_cpu_count = self.curr_core_cpus.len().max(1) as f64;
        let ticks_denom = elapsed_secs * procfs::clk_tck() as f64 * total_cpu_count;

        let mut next_ticks = HashMap::with_capacity(scanned.len());
        let mut next_io = HashMap::with_capacity(scanned.len());
        let mut next_cpu = HashMap::with_capacity(scanned.len());

        let filter_lower = filter.trim().to_lowercase();
        let has_filter = !filter_lower.is_empty();
        let mut processes = Vec::with_capacity(scanned.len());

        for proc_item in scanned {
            let pid = proc_item.pid;
            let cpu = if let Some(&prev_t) = self.prev_proc_ticks.get(&pid) {
                let delta = proc_item.stat.total_ticks.saturating_sub(prev_t) as f64;
                ((delta / ticks_denom) * 100.0).clamp(0.0, 100.0) as f32
            } else {
                0.0
            };

            let (read_rate, write_rate) = if let Some(&(prev_r, prev_w)) = self.prev_proc_io.get(&pid) {
                (
                    calculate_rate(proc_item.io_read_bytes, prev_r, elapsed_secs),
                    calculate_rate(proc_item.io_write_bytes, prev_w, elapsed_secs),
                )
            } else {
                (0, 0)
            };

            next_ticks.insert(pid, proc_item.stat.total_ticks);
            next_io.insert(pid, (proc_item.io_read_bytes, proc_item.io_write_bytes));
            next_cpu.insert(pid, cpu);

            if !show_system && is_system_process(&proc_item.stat.comm) {
                continue;
            }

            if has_filter {
                let matches_name = proc_item.stat.comm.to_lowercase().contains(&filter_lower);
                let matches_pid = !matches_name && pid.to_string().contains(&filter_lower);
                if !matches_name && !matches_pid {
                    continue;
                }
            }

            let user = self
                .users_cache
                .get_user_by_uid(proc_item.uid)
                .unwrap_or_else(|| proc_item.uid.to_string());

            let mut status = procfs::proc_state_name(proc_item.stat.state).to_string();
            if pid.as_u32() == self.self_pid || cpu > 0.0 {
                status = "Running".to_string();
            }

            let parent_pid = if proc_item.stat.ppid > 0 {
                Some(proc_item.stat.ppid.to_string())
            } else {
                None
            };

            processes.push(ProcessInfo {
                pid: pid.to_string(),
                name: proc_item.stat.comm,
                cpu,
                mem: proc_item.stat.rss_bytes,
                disk_read: read_rate,
                disk_write: write_rate,
                user,
                status,
                parent_pid,
                tree_prefix: String::new(),
                nice: proc_item.stat.nice,
                cgroup_unit: proc_item.cgroup_unit,
            });
        }

        self.prev_proc_ticks = next_ticks;
        self.prev_proc_io = next_io;
        self.last_proc_cpu = next_cpu;
        processes
    }
    
    pub fn get_detailed_process(&self, pid: Pid) -> Option<DetailedProcessInfo> {
        let (uid, stat, command, cwd, environ) = procfs::read_process_single(pid)?;
        let start_epoch = self
            .boot_time
            .saturating_add(stat.starttime_ticks / procfs::clk_tck().max(1));
        let start_time = if start_epoch > 0 {
            format_timestamp(start_epoch as i64, "%Y-%m-%d %H:%M:%S")
        } else {
            "Unknown".to_string()
        };

        let user = self
            .users_cache
            .get_user_by_uid(uid)
            .unwrap_or_else(|| uid.to_string());

        let (total_fds, sockets, pipes, fds) = Self::get_process_fds(pid);
        let thread_list = Self::get_process_threads(pid);
        let (io_read_bytes, io_write_bytes, _, _) = procfs::read_process_io(pid);
        let cpu_usage = self.last_proc_cpu.get(&pid).copied().unwrap_or(0.0);
        let cgroup_unit = procfs::read_process_cgroup_unit(pid);
        let smaps = procfs::read_process_smaps_rollup(pid);
        let cpu_affinity = crate::sys::ffi::get_cpu_affinity(pid)
            .ok()
            .map(|cpus| crate::sys::ffi::format_cpu_list(&cpus));
        let io_priority = crate::sys::ffi::get_io_priority(pid)
            .map(|(cls, lvl)| crate::sys::ffi::format_io_priority(cls, lvl));

        Some(DetailedProcessInfo {
            pid: pid.to_string(),
            name: stat.comm,
            user,
            status: procfs::proc_state_name(stat.state).to_string(),
            cpu_usage,
            memory_rss: stat.rss_bytes,
            memory_vms: stat.vsize_bytes,
            command,
            start_time,
            parent: if stat.ppid > 0 { Some(stat.ppid.to_string()) } else { None },
            environ,
            threads: if !thread_list.is_empty() {
                thread_list.len() as u32
            } else {
                stat.num_threads
            },
            file_descriptors: total_fds,
            sockets_count: sockets,
            pipes_count: pipes,
            fds,
            thread_list,
            cwd,
            nice: stat.nice,
            io_read_bytes,
            io_write_bytes,
            cgroup_unit,
            memory_pss: smaps.map(|s| s.pss_bytes),
            memory_shared: smaps.map(|s| s.shared_bytes),
            memory_private_dirty: smaps.map(|s| s.private_dirty_bytes),
            memory_swap: smaps.map(|s| s.swap_bytes),
            cpu_affinity,
            io_priority,
        })
    }
    
    pub fn get_cores(&self) -> Vec<CoreInfo> {
        let (core_sensors, cpu_temp_global) = sysfs::extract_cpu_core_temps(&self.cached_sensors);

        let core_count = self.curr_core_cpus.len().max(1);
        self.curr_core_cpus.iter().enumerate().map(|(i, curr_ticks)| {
            let usage = self.prev_core_cpus
                .get(i)
                .map(|prev| curr_ticks.usage_percent(prev))
                .unwrap_or(0.0);
            let fallback_mhz = self.cpu_info.core_mhz.get(i).copied().unwrap_or(0);
            let freq = procfs::read_core_freq_mhz(i, fallback_mhz);

            let temp = core_sensors.iter()
                .find(|(id, _)| *id == i as u32)
                .map(|(_, t)| *t)
                .or_else(|| {
                    if !core_sensors.is_empty() {
                        let sensor_idx = (i * core_sensors.len() / core_count).min(core_sensors.len() - 1);
                        Some(core_sensors[sensor_idx].1)
                    } else {
                        None
                    }
                })
                .or(cpu_temp_global);

            CoreInfo {
                usage,
                freq,
                temp,
            }
        }).collect()
    }
    
    pub fn get_disks(&mut self) -> Vec<DetailedDiskInfo> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_disk_update).as_secs_f64().max(0.1);
        
        let current_disk_stats = procfs::read_diskstats();
        let mounts = procfs::read_mounts();

        let result: Vec<DetailedDiskInfo> = mounts.into_iter().filter_map(|mount| {
            let fs_usage = procfs::stat_filesystem(&mount.mount_point)?;
            if fs_usage.total == 0 {
                return None;
            }
            let used = fs_usage.total.saturating_sub(fs_usage.free);
            let block_dev = mount.device.split('/').next_back().unwrap_or(&mount.device);

            let base_dev = if block_dev.starts_with("nvme") {
                if let Some(pos) = block_dev.find('p') {
                    if block_dev[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
                        &block_dev[..pos]
                    } else {
                        block_dev
                    }
                } else {
                    block_dev
                }
            } else {
                block_dev.trim_end_matches(|c: char| c.is_ascii_digit())
            };

            let temp = sysfs::find_disk_temp(&self.cached_sensors, block_dev);

            let mut health_pct: Option<u8> = None;
            let mut power_cycles: Option<u64> = None;

            if base_dev.starts_with("nvme") {
                if let Ok(val) = fs::read_to_string(format!("/sys/block/{}/device/percentage_used", base_dev)) {
                    if let Ok(pct) = val.trim().parse::<u8>() {
                        health_pct = Some(100u8.saturating_sub(pct));
                    }
                }
                if let Ok(val) = fs::read_to_string(format!("/sys/block/{}/device/power_cycles", base_dev)) {
                    power_cycles = val.trim().parse::<u64>().ok();
                }
            }

            let is_ssd = fs::read_to_string(format!("/sys/block/{}/queue/rotational", base_dev))
                .ok()
                .and_then(|v| v.trim().parse::<u8>().ok())
                .map(|v| v == 0);
            let io_scheduler = sysfs::read_block_scheduler(base_dev);

            let (read_rate, write_rate, await_ms, busy_pct) =
                if let (Some(curr), Some(prev)) = (
                    current_disk_stats.get(block_dev),
                    self.prev_disk_stats.get(block_dev),
                ) {
                    procfs::compute_disk_io_metrics(curr, prev, elapsed)
                } else {
                    (0, 0, 0.0, 0.0)
                };

            Some(DetailedDiskInfo {
                name: mount.mount_point,
                device: mount.device,
                fs: mount.fs_type,
                total: fs_usage.total,
                free: fs_usage.free,
                used,
                read_rate,
                write_rate,
                await_ms,
                busy_pct,
                is_ssd,
                temp,
                health_pct,
                power_cycles,
                inodes_total: fs_usage.inodes_total,
                inodes_used: fs_usage.inodes_used,
                io_scheduler,
                mount_options: Some(mount.options),
            })
        }).collect();

        self.prev_disk_stats = current_disk_stats;
        self.last_disk_update = now;
        result
    }
    
    pub fn get_networks(&mut self) -> Vec<DetailedNetInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_net_update).as_secs_f64().max(0.1);
        self.last_net_update = now;
        
        let mut current_net_usage = HashMap::new();
        let net_devs = procfs::read_net_dev();
        let networks: Vec<DetailedNetInfo> = net_devs
            .into_iter()
            .map(|dev| {
                let (down_rate, up_rate) = if let Some(prev) = self.prev_net_usage.get(&dev.name) {
                    let rx_rate = calculate_rate(dev.rx_bytes, prev.rx, elapsed_secs);
                    let tx_rate = calculate_rate(dev.tx_bytes, prev.tx, elapsed_secs);
                    (rx_rate, tx_rate)
                } else {
                    (0, 0)
                };
                
                current_net_usage.insert(
                    dev.name.clone(),
                    NetworkStats {
                        rx: dev.rx_bytes,
                        tx: dev.tx_bytes,
                    }
                );

                let is_up = sysfs::is_net_interface_up(&dev.name);
                
                DetailedNetInfo {
                    name: dev.name,
                    down_rate,
                    up_rate,
                    total_down: dev.rx_bytes,
                    total_up: dev.tx_bytes,
                    packets_rx: dev.rx_packets,
                    packets_tx: dev.tx_packets,
                    is_up,
                }
            })
            .collect();
        
        self.prev_net_usage = current_net_usage;
        networks
    }
    
    pub fn get_global_usage(&mut self, total_net_down: u64, total_net_up: u64, 
                           total_disk_read: u64, total_disk_write: u64,
                           gpu_util: Option<u32>) -> GlobalUsage {
        let load = procfs::read_load_average();
        let (uptime, _) = procfs::read_uptime_and_boot_time();

        if self.memory_details_cache.is_none() {
            self.memory_details_cache = Some(self.get_memory_details());
        }

        GlobalUsage {
            cpu: self.curr_global_cpu.usage_percent(&self.prev_global_cpu),
            mem_used: self.mem_info.used(),
            mem_total: self.mem_info.total,
            mem_cached: self.mem_info.cached_bytes(),
            swap_used: self.mem_info.swap_used(),
            swap_total: self.mem_info.swap_total,
            gpu_util,
            net_down: total_net_down,
            net_up: total_net_up,
            disk_read: total_disk_read,
            disk_write: total_disk_write,
            load_average: load,
            uptime,
            mem_details: self.memory_details_cache.clone(),
            ..Default::default()
        }
    }
    
    pub fn get_temperatures(&self) -> SystemTemperatures {
        let (_, cpu_temp) = sysfs::extract_cpu_core_temps(&self.cached_sensors);
        SystemTemperatures { cpu_temp }
    }

    pub fn get_sensors(&self) -> Vec<SensorInfo> {
        self.cached_sensors.clone()
    }

    pub fn get_global_disk_io(&mut self) -> (u64, u64) {
        let current_stats = procfs::read_diskstats();
        let mut total_read_rate = 0;
        let mut total_write_rate = 0;
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_disk_update).as_secs_f64().max(0.1);
        
        for (dev, curr) in &current_stats {
            if let Some(prev) = self.prev_disk_stats.get(dev) {
                let read_rate = (curr.read_bytes.saturating_sub(prev.read_bytes) as f64 / elapsed) as u64;
                let write_rate = (curr.write_bytes.saturating_sub(prev.write_bytes) as f64 / elapsed) as u64;
                total_read_rate += read_rate;
                total_write_rate += write_rate;
            }
        }
        
        self.prev_disk_stats = current_stats;
        self.last_disk_update = now;
        
        (total_read_rate, total_write_rate)
    }
    
    pub fn calculate_total_network_io(&self, networks: &[DetailedNetInfo]) -> (u64, u64) {
        let total_down = networks.iter().map(|n| n.down_rate).sum();
        let total_up = networks.iter().map(|n| n.up_rate).sum();
        (total_down, total_up)
    }

    pub fn get_process_fds(pid: Pid) -> (Option<u32>, Option<u32>, Option<u32>, Vec<ProcessFdInfo>) {
        let fd_dir = format!("/proc/{}/fd", pid);
        if let Ok(entries) = fs::read_dir(fd_dir) {
            let mut total = 0;
            let mut sockets = 0;
            let mut pipes = 0;
            let mut fds = Vec::new();

            for entry in entries.flatten() {
                total += 1;
                let fd_num = entry.file_name().to_string_lossy().to_string();
                if let Ok(target) = fs::read_link(entry.path()) {
                    let target_str = target.to_string_lossy().to_string();
                    let fd_type = if target_str.starts_with("socket:[") {
                        sockets += 1;
                        "Socket".to_string()
                    } else if target_str.starts_with("pipe:[") {
                        pipes += 1;
                        "Pipe".to_string()
                    } else if target_str.starts_with("anon_inode:[") {
                        "AnonInode".to_string()
                    } else if target_str.starts_with("/dev/") {
                        "Device".to_string()
                    } else if target_str.starts_with("/proc/") {
                        "Procfs".to_string()
                    } else if target_str.starts_with('/') {
                        "File".to_string()
                    } else {
                        "Other".to_string()
                    };
                    fds.push(ProcessFdInfo {
                        fd: fd_num,
                        fd_type,
                        target: target_str,
                    });
                }
            }
            fds.sort_by(|a, b| {
                let a_num = a.fd.parse::<u32>().unwrap_or(u32::MAX);
                let b_num = b.fd.parse::<u32>().unwrap_or(u32::MAX);
                a_num.cmp(&b_num)
            });
            (Some(total), Some(sockets), Some(pipes), fds)
        } else {
            (None, None, None, Vec::new())
        }
    }

    pub fn get_process_threads(pid: Pid) -> Vec<ProcessThreadInfo> {
        let task_dir = format!("/proc/{}/task", pid);
        let mut thread_list = Vec::new();
        if let Ok(entries) = fs::read_dir(task_dir) {
            for entry in entries.flatten() {
                let tid = entry.file_name().to_string_lossy().to_string();
                let comm_path = entry.path().join("comm");
                let comm = fs::read_to_string(comm_path).unwrap_or_else(|_| "unknown".to_string()).trim().to_string();
                
                let status = if let Ok(stat_content) = fs::read_to_string(entry.path().join("stat")) {
                    if let Some(idx) = stat_content.rfind(')') {
                        let rest = stat_content[idx + 1..].trim_start();
                        match rest.chars().next() {
                            Some('R') => "Running",
                            Some('S') => "Sleeping",
                            Some('D') => "Disk Sleep",
                            Some('Z') => "Zombie",
                            Some('T') => "Stopped",
                            Some('t') => "Tracing Stop",
                            Some('X') | Some('x') => "Dead",
                            Some('K') => "Wakekill",
                            Some('W') => "Waking",
                            Some('P') => "Parked",
                            Some('I') => "Idle",
                            _ => "Unknown",
                        }.to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                thread_list.push(ProcessThreadInfo {
                    tid,
                    name: comm,
                    status,
                });
            }
        }
        thread_list.sort_by(|a, b| {
            let a_id = a.tid.parse::<u32>().unwrap_or(0);
            let b_id = b.tid.parse::<u32>().unwrap_or(0);
            a_id.cmp(&b_id)
        });
        thread_list
    }

    #[cfg(test)]
    pub fn get_process_io(pid: Pid) -> (u64, u64, u64, u64) {
        procfs::read_process_io(pid)
    }

    #[cfg(test)]
    pub fn get_mount_inodes(mount_point: &str) -> (Option<u64>, Option<u64>) {
        if let Some(usage) = procfs::stat_filesystem(mount_point) {
            (usage.inodes_total, usage.inodes_used)
        } else {
            (None, None)
        }
    }

    #[cfg(test)]
    pub fn get_mount_options() -> HashMap<String, String> {
        procfs::read_mounts()
            .into_iter()
            .map(|m| (m.mount_point, m.options))
            .collect()
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sort_processes(processes: &mut [ProcessInfo], sort_by: &ProcessSortBy, ascending: bool, total_memory: u64) {
    match sort_by {
        ProcessSortBy::Cpu => {
            processes.sort_by(|a, b| {
                let cmp = a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal);
                if ascending { cmp } else { cmp.reverse() }
            });
        },
        ProcessSortBy::Memory => {
            processes.sort_by(|a, b| {
                let cmp = a.mem.cmp(&b.mem);
                if ascending { cmp } else { cmp.reverse() }
            });
        },
        ProcessSortBy::Name => {
            processes.sort_by(|a, b| {
                let cmp = a.name.cmp(&b.name);
                if ascending { cmp } else { cmp.reverse() }
            });
        },
        ProcessSortBy::General => {
            processes.sort_by(|a, b| {
                let a_score = a.cpu + (a.mem as f32 / total_memory as f32 * 100.0);
                let b_score = b.cpu + (b.mem as f32 / total_memory as f32 * 100.0);
                let cmp = a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal);
                if ascending { cmp } else { cmp.reverse() }
            });
        },
    }
}

pub fn build_process_tree(
    processes: &mut Vec<ProcessInfo>,
    sort_by: &ProcessSortBy,
    ascending: bool,
    total_memory: u64,
) {
    if processes.is_empty() {
        return;
    }

    sort_processes(processes, sort_by, ascending, total_memory);

    let mut pid_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in processes.iter() {
        pid_set.insert(p.pid.clone());
    }

    let mut children_map: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    let mut roots: Vec<usize> = Vec::new();

    for (idx, p) in processes.iter().enumerate() {
        let is_root = match &p.parent_pid {
            None => true,
            Some(ppid) => ppid == "0" || ppid == &p.pid || !pid_set.contains(ppid),
        };

        if is_root {
            roots.push(idx);
        } else if let Some(ppid) = &p.parent_pid {
            children_map.entry(ppid.clone()).or_default().push(idx);
        }
    }

    let mut ordered: Vec<ProcessInfo> = Vec::with_capacity(processes.len());
    let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();

    #[allow(clippy::too_many_arguments)]
    fn traverse(
        idx: usize,
        processes: &[ProcessInfo],
        children_map: &std::collections::HashMap<String, Vec<usize>>,
        visited: &mut std::collections::HashSet<usize>,
        ordered: &mut Vec<ProcessInfo>,
        prefix: &str,
        is_last: bool,
        is_root: bool,
    ) {
        if !visited.insert(idx) {
            return;
        }

        let p = &processes[idx];
        let mut item = p.clone();

        if is_root {
            item.tree_prefix = String::new();
        } else {
            let branch = if is_last { "└─ " } else { "├─ " };
            item.tree_prefix = format!("{}{}", prefix, branch);
        }

        ordered.push(item);

        if let Some(children) = children_map.get(&p.pid) {
            let child_prefix = if is_root {
                ""
            } else if is_last {
                "   "
            } else {
                "│  "
            };
            let next_prefix = format!("{}{}", prefix, child_prefix);

            let total_children = children.len();
            for (c_idx, &child_i) in children.iter().enumerate() {
                let last_child = c_idx == total_children - 1;
                traverse(
                    child_i,
                    processes,
                    children_map,
                    visited,
                    ordered,
                    &next_prefix,
                    last_child,
                    false,
                );
            }
        }
    }

    let root_count = roots.len();
    for (r_idx, &root_i) in roots.iter().enumerate() {
        traverse(
            root_i,
            processes,
            &children_map,
            &mut visited,
            &mut ordered,
            "",
            r_idx == root_count - 1,
            true,
        );
    }

    for (i, p) in processes.iter().enumerate() {
        if !visited.contains(&i) {
            let mut item = p.clone();
            item.tree_prefix = String::new();
            ordered.push(item);
        }
    }

    *processes = ordered;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_creation() {
        let monitor = SystemMonitor::new();
        assert!(!monitor.curr_core_cpus.is_empty());
        assert!(monitor.get_total_memory() > 0);
    }
    
    #[test]
    fn test_process_sorting() {
        let mut processes = vec![
            ProcessInfo {
                pid: "1".to_string(),
                name: "init".to_string(),
                cpu: 1.0,
                mem: 1024,
                disk_read: 0,
                disk_write: 0,
                user: "root".to_string(),
                status: "Running".to_string(),
                parent_pid: None,
                tree_prefix: String::new(),
                nice: 0,
                cgroup_unit: None,
            },
            ProcessInfo {
                pid: "2".to_string(),
                name: "kthreadd".to_string(),
                cpu: 5.0,
                mem: 2048,
                disk_read: 0,
                disk_write: 0,
                user: "root".to_string(),
                status: "Running".to_string(),
                parent_pid: None,
                tree_prefix: String::new(),
                nice: 0,
                cgroup_unit: None,
            },
        ];
        
        sort_processes(&mut processes, &ProcessSortBy::Cpu, false, 8192 * 1024 * 1024);
        assert_eq!(processes[0].name, "kthreadd");
        
        sort_processes(&mut processes, &ProcessSortBy::Memory, false, 8192 * 1024 * 1024);
        assert_eq!(processes[0].name, "kthreadd");
    }

    #[test]
    fn test_process_tree() {
        let mut processes = vec![
            ProcessInfo {
                pid: "1".to_string(),
                name: "systemd".to_string(),
                cpu: 0.1,
                mem: 1024,
                disk_read: 0,
                disk_write: 0,
                user: "root".to_string(),
                status: "Running".to_string(),
                parent_pid: None,
                tree_prefix: String::new(),
                nice: 0,
                cgroup_unit: None,
            },
            ProcessInfo {
                pid: "100".to_string(),
                name: "child_proc".to_string(),
                cpu: 0.2,
                mem: 2048,
                disk_read: 0,
                disk_write: 0,
                user: "root".to_string(),
                status: "Running".to_string(),
                parent_pid: Some("1".to_string()),
                tree_prefix: String::new(),
                nice: 0,
                cgroup_unit: None,
            },
        ];

        build_process_tree(&mut processes, &ProcessSortBy::Cpu, true, 8192 * 1024 * 1024);
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].name, "systemd");
        assert_eq!(processes[0].tree_prefix, "");
        assert_eq!(processes[1].name, "child_proc");
        assert_eq!(processes[1].tree_prefix, "└─ ");
    }

    #[test]
    fn test_process_fds_and_threads() {
        let pid = Pid::from(std::process::id());
        let (total_fds, sockets, pipes, fds) = SystemMonitor::get_process_fds(pid);
        assert!(total_fds.is_some());
        assert!(total_fds.unwrap() > 0);
        assert!(!fds.is_empty());
        assert!(sockets.is_some());
        assert!(pipes.is_some());

        let threads = SystemMonitor::get_process_threads(pid);
        assert!(!threads.is_empty());
        assert_eq!(threads[0].tid, std::process::id().to_string());
    }

    #[test]
    fn test_mount_inodes_and_process_io() {
        let (total, used) = SystemMonitor::get_mount_inodes("/");
        assert!(total.is_some(), "statvfs on root / should succeed");
        let total = total.unwrap();
        let used = used.unwrap();
        assert!(total > 0, "root filesystem should have total inodes > 0");
        assert!(total >= used, "total inodes must be >= used inodes");

        let mount_opts = SystemMonitor::get_mount_options();
        assert!(!mount_opts.is_empty(), "mount options map should not be empty");

        let pid = Pid::from(std::process::id());
        let (_read_bytes, _write_bytes, rchar, _wchar) = SystemMonitor::get_process_io(pid);
        assert!(rchar > 0);
    }
}