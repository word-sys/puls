use std::collections::HashMap;
use std::time::Instant;
use std::fs;
use sysinfo::{DiskUsage, Networks, Pid, System, Components};

use crate::types::*;
use crate::utils::*;

extern "C" {
    fn getpriority(which: i32, who: u32) -> i32;
}

#[derive(Debug, Clone, Default)]
struct DiskStatsData {
    read_bytes: u64,
    write_bytes: u64,
    read_ops: u64,
    write_ops: u64,
}

pub struct SystemMonitor {
    system: System,
    components: Components,
    users_cache: UsersCache,
    prev_disk_usage: HashMap<Pid, DiskUsage>,
    prev_net_usage: HashMap<String, NetworkStats>,
    prev_disk_stats: HashMap<String, DiskStatsData>,
    last_update: Instant,
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
        let mut system = System::new_all();
        system.refresh_all();
        
        let components = Components::new_with_refreshed_list();
        
        Self {
            system,
            components,
            users_cache: UsersCache::new(),
            prev_disk_usage: HashMap::new(),
            prev_net_usage: HashMap::new(),
            prev_disk_stats: HashMap::new(),
            last_update: Instant::now(),
            self_pid: std::process::id(),
            memory_details_cache: None,
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        let mut info = vec![
            ("OS".into(), System::long_os_version().unwrap_or_default()),
            ("Kernel".into(), System::kernel_version().unwrap_or_default()),
            ("Hostname".into(), System::host_name().unwrap_or_default()),
            ("CPU".into(), self.system.cpus().first().map_or("N/A".into(), |c| c.brand().to_string())),
            ("Cores".into(), format!("{} Physical / {} Logical", 
                self.system.physical_core_count().unwrap_or(0), 
                self.system.cpus().len())),
            ("Total Memory".into(), format_size(self.system.total_memory())),
        ];

        if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
            let mut cache_size = "N/A".to_string();
            let mut bogomips = "N/A".to_string();
            let mut vendor = "N/A".to_string();
            let mut virtualization = "Disabled/None".to_string();
            let mut family = "N/A".to_string();

            for line in content.lines() {
                if line.starts_with("cache size") {
                    cache_size = line.split(':').next_back().unwrap_or("").trim().to_string();
                } else if line.starts_with("bogomips") {
                    bogomips = line.split(':').next_back().unwrap_or("").trim().to_string();
                } else if line.starts_with("vendor_id") {
                    vendor = line.split(':').next_back().unwrap_or("").trim().to_string();
                } else if line.starts_with("cpu family") {
                    family = line.split(':').next_back().unwrap_or("").trim().to_string();
                } else if line.starts_with("flags") && virtualization == "Disabled/None" {
                    let flags = line.split(':').next_back().unwrap_or("");
                    if flags.contains("vmx") {
                        virtualization = "Intel VT-x".to_string();
                    } else if flags.contains("svm") {
                        virtualization = "AMD-V".to_string();
                    }
                }
            }
            
            let (l1, l2, l3) = get_cpu_caches();
            info.push(("Vendor".into(), vendor));
            info.push(("Family".into(), family));
            info.push(("L1 Cache".into(), l1));
            info.push(("L2 Cache".into(), l2));
            if l3 != "N/A" {
                info.push(("L3 Cache".into(), l3));
            } else {
                info.push(("L3 Cache".into(), cache_size));
            }
            info.push(("BogoMIPS".into(), bogomips));
            info.push(("Virtualization".into(), virtualization));
        }

        info.extend(vec![
            ("Boot Time".into(), {
                let boot_time = System::boot_time();
                if boot_time > 0 {
                    format_timestamp(boot_time as i64, "%Y-%m-%d %H:%M:%S")
                } else {
                    "Unknown".to_string()
                }
            }),
            ("Uptime".into(), {
                let boot_time = System::boot_time(); if boot_time > 0 {
                    let uptime = current_timestamp().saturating_sub(boot_time);
                    format_duration(uptime)
                } else {
                    "Unknown".to_string()
                }
            }),
            ("Load Average".into(), {
                let load = System::load_average();
                format!("{:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen)
            }),
        ]);
        info
    }

    pub fn get_total_memory(&self) -> u64 {
        self.system.total_memory()
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

        if let Ok(output) = std::process::Command::new("dmidecode")
            .args(["-t", "17"])
            .output() 
        {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("Type:") {
                        let t = line.split(':').next_back().unwrap_or("").trim();
                        if !t.is_empty() && !["Unknown", "Other", "<OUT OF SPEC>"].contains(&t) {
                            mem_gen = t.to_string();
                        }
                    } else if line.starts_with("Speed:") {
                        let s = line.split(':').next_back().unwrap_or("").trim();
                        if !s.is_empty() && !["Unknown", "Unknown Speed", "0 MT/s", "0 MHz"].contains(&s) {
                            mem_speed = s.to_string();
                        }
                    } else if line.starts_with("Form Factor:") {
                        let f = line.split(':').next_back().unwrap_or("").trim();
                        if !f.is_empty() && f != "Unknown" {
                            mem_type = f.to_string();
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

        if let Some(t) = self.components.iter().find(|c| {
            let label = c.label().to_lowercase();
            label.contains("mem") || label.contains("dimm") || label.contains("dram")
        }).and_then(|c| c.temperature()) {
            mem_temp = format!("{:.1}°C", t);
        }

        (mem_type, mem_gen, mem_speed, mem_temp)
    }
    
    pub fn refresh_core_metrics(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.components.refresh(true);
    }

    pub fn update_processes(&mut self, show_system: bool, filter: &str) -> Vec<ProcessInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;
        
        let process_refresh_kind = sysinfo::ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage()
            .with_user(sysinfo::UpdateKind::OnlyIfNotSet)
            .with_exe(sysinfo::UpdateKind::OnlyIfNotSet)
            .with_cmd(sysinfo::UpdateKind::OnlyIfNotSet)
            .with_environ(sysinfo::UpdateKind::Never);
        
        self.system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All, 
            true, 
            process_refresh_kind
        );
        
        let total_cpu_count = self.system.cpus().len() as f32;
        let mut current_disk_usage = HashMap::new();
        let filter_lower = filter.trim().to_lowercase();
        let has_filter = !filter_lower.is_empty();

        let processes: Vec<ProcessInfo> = self.system.processes()
            .iter()
            .filter(|(_pid, process)| {
                /*
                if pid.as_u32() == self.self_pid {
                    return false;
                }
                */
                
                let proc_name = process.name().to_string_lossy();
                if !show_system && is_system_process(&proc_name) {
                    return false;
                }
                
                if has_filter {
                    let matches_name = proc_name.to_lowercase().contains(&filter_lower);
                    let matches_pid = if !matches_name {
                        process.pid().to_string().contains(&filter_lower)
                    } else {
                        false
                    };
                    if !matches_name && !matches_pid {
                        return false;
                    }
                }
                
                true
            })
            .map(|(pid, process)| {
                let disk_usage = process.disk_usage();
                let (read_rate, write_rate) = if let Some(prev) = self.prev_disk_usage.get(pid) {
                    let read_bytes = calculate_rate(
                        disk_usage.total_read_bytes,
                        prev.total_read_bytes,
                        elapsed_secs
                    );
                    let written_bytes = calculate_rate(
                        disk_usage.total_written_bytes,
                        prev.total_written_bytes,
                        elapsed_secs
                    );
                    (read_bytes, written_bytes)
                } else {
                    (0, 0)
                };
                
                current_disk_usage.insert(*pid, disk_usage);
                
                let user = process.user_id()
                    .and_then(|uid| self.users_cache.get_user_by_uid(**uid))
                    .unwrap_or_else(|| "N/A".to_string());
                
                let raw_cpu = process.cpu_usage();
                let normalized_cpu = (raw_cpu / total_cpu_count).clamp(0.0, 100.0);
                
                let mut status = process.status().to_string();
                
                if pid.as_u32() == self.self_pid || normalized_cpu > 0.0 {
                     status = "Running".to_string();
                }

                let nice = Self::get_process_nice(*pid);

                ProcessInfo {
                    pid: pid.to_string(),
                    name: process.name().to_string_lossy().to_string(),
                    cpu: normalized_cpu,
                    mem: process.memory(),
                    disk_read: read_rate,
                    disk_write: write_rate,
                    user,
                    status,
                    parent_pid: process.parent().map(|p| p.to_string()),
                    tree_prefix: String::new(),
                    nice,
                }
            })
            .collect();
        
        self.prev_disk_usage = current_disk_usage;
        processes
    }
    
    pub fn get_detailed_process(&self, pid: Pid) -> Option<DetailedProcessInfo> {
        self.system.process(pid).map(|process| {
            let start_time = format_timestamp(process.start_time() as i64, "%Y-%m-%d %H:%M:%S");
            
            let user = process.user_id()
                .and_then(|uid| self.users_cache.get_user_by_uid(**uid))
                .unwrap_or_else(|| "N/A".to_string());
            
            let (total_fds, sockets, pipes, fds) = Self::get_process_fds(pid);
            let open_files = fds.iter().map(|f| f.target.clone()).collect();
            let thread_list = Self::get_process_threads(pid);
            let nice = Self::get_process_nice(pid);
            let (io_read_bytes, io_write_bytes, io_read_chars, io_write_chars) = Self::get_process_io(pid);

            let mut environ: Vec<String> = process.environ().iter().map(|s| s.to_string_lossy().to_string()).collect();
            if environ.is_empty() {
                if let Ok(bytes) = fs::read(format!("/proc/{}/environ", pid)) {
                    environ = bytes.split(|&b| b == 0)
                        .filter(|slice| !slice.is_empty())
                        .map(|slice| String::from_utf8_lossy(slice).to_string())
                        .collect();
                }
            }
            environ.sort();

            DetailedProcessInfo {
                pid: process.pid().to_string(),
                name: process.name().to_string_lossy().to_string(),
                user,
                status: process.status().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_rss: process.memory(),
                memory_vms: process.virtual_memory(),
                command: process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect::<Vec<String>>().join(" "),
                start_time,
                parent: process.parent().map(|p| p.to_string()),
                environ,
                threads: if !thread_list.is_empty() { thread_list.len() as u32 } else { process.tasks().map(|t| t.len() as u32).unwrap_or(0) },
                file_descriptors: total_fds,
                sockets_count: sockets,
                pipes_count: pipes,
                open_files,
                fds,
                thread_list,
                cwd: process.cwd().map(|p| p.to_string_lossy().into_owned()),
                nice,
                io_read_bytes,
                io_write_bytes,
                io_read_chars,
                io_write_chars,
            }
        })
    }
    
    pub fn get_cores(&self) -> Vec<CoreInfo> {
        let components = &self.components;
        
        let mut core_sensors: Vec<(u32, f32)> = components.iter()
            .filter(|c| {
                let label = c.label().to_lowercase();
                label.contains("core") && !label.contains("package")
            })
            .filter_map(|c| {
                let label = c.label().to_lowercase();
                let id = label.split_whitespace()
                    .filter_map(|s| s.parse::<u32>().ok())
                    .next()?;
                let temp = c.temperature()?;
                Some((id, temp))
            })
            .collect();
        
        core_sensors.sort_by_key(|(id, _)| *id);

        let cpu_temp_global = components.iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("tctl") || label.contains("package") || label.contains("tdie")
            })
            .and_then(|c| c.temperature())
            .or_else(|| {
                core_sensors.first().map(|(_, t)| *t)
            })
            .or_else(Self::read_cpu_temp_from_hwmon);

        self.system.cpus().iter().enumerate().map(|(i, cpu)| {
            let temp = core_sensors.iter()
                .find(|(id, _)| *id == i as u32)
                .map(|(_, t)| *t)
                .or_else(|| {
                    if !core_sensors.is_empty() {
                        let sensor_idx = (i * core_sensors.len() / self.system.cpus().len()).min(core_sensors.len() - 1);
                        Some(core_sensors[sensor_idx].1)
                    } else {
                        None
                    }
                })
                .or(cpu_temp_global);

            CoreInfo {
                usage: cpu.cpu_usage(),
                freq: cpu.frequency(),
                temp,
            }
        }).collect()
    }
    
    pub fn get_disks(&mut self) -> Vec<DetailedDiskInfo> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        
        let current_disk_stats = self.parse_disk_stats();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        
        let generic_nvme_temp = if let Ok(hwmon_entries) = fs::read_dir("/sys/class/hwmon") {
            hwmon_entries.flatten().find_map(|entry| {
                let path = entry.path();
                if let Ok(name) = fs::read_to_string(path.join("name")) {
                    if name.trim() == "nvme" {
                        if let Ok(val) = fs::read_to_string(path.join("temp1_input")) {
                            if let Ok(mdeg) = val.trim().parse::<f32>() {
                                return Some(mdeg / 1000.0);
                            }
                        }
                    }
                }
                None
            })
        } else {
            None
        };

        let mount_options_map = Self::get_mount_options();

        let result: Vec<DetailedDiskInfo> = disks.iter().map(|disk| {
            let used = disk.total_space().saturating_sub(disk.available_space());
            let dev_path = disk.name().to_string_lossy();
            let block_dev = dev_path.split('/').next_back().unwrap_or(&dev_path);
            let mount_point = disk.mount_point().to_string_lossy();
            let (inodes_total, inodes_free, inodes_used) = Self::get_mount_inodes(&mount_point);
            let mount_options = mount_options_map.get(mount_point.as_ref()).cloned();
            
            let base_dev = if block_dev.starts_with("nvme") {
                if let Some(pos) = block_dev.find('p') {
                    if block_dev[pos+1..].chars().all(|c| c.is_ascii_digit()) {
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

            let temp = self.components.iter()
                .find(|c| {
                    let label = c.label().to_lowercase();
                    label.contains(block_dev) || block_dev.contains(&label)
                })
                .and_then(|c| c.temperature())
                .or(generic_nvme_temp);

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

            let mut read_rate = 0;
            let mut write_rate = 0;
            let mut read_ops = 0;
            let mut write_ops = 0;

            if let Some(curr) = current_disk_stats.get(block_dev) {
                if let Some(prev) = self.prev_disk_stats.get(block_dev) {
                    read_rate = ((curr.read_bytes.saturating_sub(prev.read_bytes)) as f64 / elapsed) as u64;
                    write_rate = ((curr.write_bytes.saturating_sub(prev.write_bytes)) as f64 / elapsed) as u64;
                    read_ops = ((curr.read_ops.saturating_sub(prev.read_ops)) as f64 / elapsed) as u64;
                    write_ops = ((curr.write_ops.saturating_sub(prev.write_ops)) as f64 / elapsed) as u64;
                }
            }
            
            DetailedDiskInfo {
                name: disk.mount_point().to_string_lossy().into_owned(),
                device: dev_path.into_owned(),
                fs: disk.file_system().to_string_lossy().to_string(),
                total: disk.total_space(),
                free: disk.available_space(),
                used,
                read_rate,
                write_rate,
                read_ops,
                write_ops,
                is_ssd,
                temp,
                health_pct,
                power_cycles,
                inodes_total,
                inodes_free,
                inodes_used,
                mount_options,
            }
        }).collect();

        self.prev_disk_stats = current_disk_stats;
        self.last_update = now;
        result
    }
    
    pub fn get_networks(&mut self) -> Vec<DetailedNetInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        
        let mut current_net_usage = HashMap::new();
        let networks = Networks::new_with_refreshed_list();
        let networks: Vec<DetailedNetInfo> = networks
            .iter()
            .map(|(interface_name, data)| {
                let (down_rate, up_rate) = if let Some(prev) = self.prev_net_usage.get(interface_name) {
                    let rx_rate = calculate_rate(data.total_received(), prev.rx, elapsed_secs);
                    let tx_rate = calculate_rate(data.total_transmitted(), prev.tx, elapsed_secs);
                    (rx_rate, tx_rate)
                } else {
                    (0, 0)
                };
                
                current_net_usage.insert(
                    interface_name.clone(),
                    NetworkStats {
                        rx: data.total_received(),
                        tx: data.total_transmitted(),
                    }
                );
                
                DetailedNetInfo {
                    name: interface_name.clone(),
                    down_rate,
                    up_rate,
                    total_down: data.total_received(),
                    total_up: data.total_transmitted(),
                    packets_rx: data.total_packets_received(),
                    packets_tx: data.total_packets_transmitted(),
                    errors_rx: data.total_errors_on_received(),
                    errors_tx: data.total_errors_on_transmitted(),
                    interface_type: "Unknown".to_string(),
                    is_up: true, 
                }
            })
            .collect();
        
        self.prev_net_usage = current_net_usage;
        networks
    }
    
    pub fn get_global_usage(&mut self, total_net_down: u64, total_net_up: u64, 
                           total_disk_read: u64, total_disk_write: u64,
                           gpu_util: Option<u32>) -> GlobalUsage {
        let load = System::load_average();
        let boot_time = System::boot_time();
        let uptime = current_timestamp().saturating_sub(boot_time);
        
        let mem_available = self.system.available_memory();
        let mem_free = self.system.free_memory();
        let mem_cached = mem_available.saturating_sub(mem_free);

        if self.memory_details_cache.is_none() {
            self.memory_details_cache = Some(self.get_memory_details());
        }

        GlobalUsage {
            cpu: self.system.global_cpu_usage(),
            mem_used: self.system.used_memory(),
            mem_total: self.system.total_memory(),
            mem_cached,
            swap_used: self.system.used_swap(),
            swap_total: self.system.total_swap(),
            gpu_util,
            net_down: total_net_down,
            net_up: total_net_up,
            disk_read: total_disk_read,
            disk_write: total_disk_write,
            disk_read_ops: 0,
            disk_write_ops: 0,
            load_average: (load.one, load.five, load.fifteen),
            uptime,
            boot_time,
            mem_details: self.memory_details_cache.clone(),
            ..Default::default()
        }
    }
    
    pub fn get_temperatures(&self) -> SystemTemperatures {
        let components = &self.components;
        let cpu_temp = components.iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("tctl") || label.contains("package") || label.contains("tdie")
            })
            .and_then(|c| c.temperature())
            .or_else(|| {
                components.iter()
                    .find(|c| c.label().to_lowercase().contains("core 0"))
                    .and_then(|c| c.temperature())
            })
            .or_else(|| {
                Self::read_cpu_temp_from_hwmon()
            });
            
        let gpu_temps: Vec<f32> = components.iter()
             .filter(|c| {
                 let label = c.label().to_lowercase();
                 label.contains("gpu") || label.contains("radeon") || label.contains("amdgpu") || label.contains("edge") || label.contains("junction")
             })
             .filter_map(|c| c.temperature())
             .collect();
             
        SystemTemperatures {
            cpu_temp,
            gpu_temps,
            motherboard_temp: None,
        }
    }

    fn read_cpu_temp_from_hwmon() -> Option<f32> {
        let hwmon_base = "/sys/class/hwmon";
        let entries = std::fs::read_dir(hwmon_base).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = std::fs::read_to_string(path.join("name")).unwrap_or_default();
            let name = name.trim().to_lowercase();
            // k10temp = AMD FX/Ryzen, coretemp = Intel, k8temp = older AMD, it87/nct* = some boards
            if name == "k10temp" || name == "coretemp" || name == "k8temp" || name == "zenpower" {
                let temp_file = path.join("temp1_input");
                if let Ok(val) = std::fs::read_to_string(&temp_file) {
                    if let Ok(millideg) = val.trim().parse::<f32>() {
                        return Some(millideg / 1000.0);
                    }
                }
            }
        }
        None
    }

    fn parse_disk_stats(&self) -> HashMap<String, DiskStatsData> {
        let mut stats = HashMap::new();
        if let Ok(content) = fs::read_to_string("/proc/diskstats") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 14 {
                    let dev_name = parts[2].to_string();
                    let reads_completed = parts[3].parse::<u64>().unwrap_or(0);
                    let sectors_read = parts[5].parse::<u64>().unwrap_or(0);
                    let writes_completed = parts[7].parse::<u64>().unwrap_or(0);
                    let sectors_written = parts[9].parse::<u64>().unwrap_or(0);
                    
                    stats.insert(dev_name, DiskStatsData {
                        read_bytes: sectors_read * 512,
                        write_bytes: sectors_written * 512,
                        read_ops: reads_completed,
                        write_ops: writes_completed,
                    });
                }
            }
        }
        stats
    }

    pub fn get_sensors(&self) -> Vec<SensorInfo> {
        let mut sensors = Vec::new();

        let mut sysinfo_max: std::collections::HashMap<String, (Option<f32>, Option<f32>)> =
            std::collections::HashMap::new();
        for c in self.components.iter() {
            sysinfo_max.insert(c.label().to_string(), (c.max(), c.critical()));
        }
        
        if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                let chip_name = std::fs::read_to_string(hwmon_path.join("name"))
                    .unwrap_or_default().trim().to_string();
                if let Ok(files) = std::fs::read_dir(&hwmon_path) {
                    let mut file_names: Vec<String> = files
                        .flatten()
                        .map(|f| f.file_name().to_string_lossy().to_string())
                        .collect();
                    file_names.sort();

                    for fname in &file_names {
                        if fname.starts_with("temp") && fname.ends_with("_input") {
                            let idx = fname
                                .trim_start_matches("temp")
                                .trim_end_matches("_input");

                            let current = std::fs::read_to_string(hwmon_path.join(fname))
                                .ok()
                                .and_then(|s| s.trim().parse::<f32>().ok())
                                .map(|v| v / 1000.0)
                                .unwrap_or(0.0);

                            let label = std::fs::read_to_string(
                                    hwmon_path.join(format!("temp{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} temp{}", chip_name, idx));

                            let hw_limit = std::fs::read_to_string(
                                    hwmon_path.join(format!("temp{}_max", idx)))
                                .ok()
                                .and_then(|s| s.trim().parse::<f32>().ok())
                                .map(|v| v / 1000.0);

                            let hw_crit = std::fs::read_to_string(
                                    hwmon_path.join(format!("temp{}_crit", idx)))
                                .ok()
                                .and_then(|s| s.trim().parse::<f32>().ok())
                                .map(|v| v / 1000.0);

                            let (sysinfo_max_val, sysinfo_crit) = sysinfo_max
                                .get(&label)
                                .copied()
                                .unwrap_or((None, None));

                            let critical = hw_crit.or(sysinfo_crit);

                            if sensors.iter().any(|s: &SensorInfo| {
                                s.sensor_type == "temp" && s.label == label
                            }) {
                                continue;
                            }

                            sensors.push(SensorInfo {
                                label,
                                chip: chip_name.clone(),
                                sensor_type: "temp".to_string(),
                                value: current as f64,
                                unit: "°C".to_string(),
                                temp: current,
                                max: sysinfo_max_val,
                                limit: hw_limit,
                                critical,
                            });
                            continue;
                        }

                        if fname.starts_with("fan") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("fan").trim_end_matches("_input");
                            let label = std::fs::read_to_string(
                                    hwmon_path.join(format!("fan{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Fan {}", chip_name, idx));
                            if let Ok(val) = std::fs::read_to_string(hwmon_path.join(fname)) {
                                if let Ok(rpm) = val.trim().parse::<f64>() {
                                    sensors.push(SensorInfo {
                                        label,
                                        chip: chip_name.clone(),
                                        sensor_type: "fan".to_string(),
                                        value: rpm,
                                        unit: "RPM".to_string(),
                                        temp: rpm as f32,
                                        max: None,
                                        limit: None,
                                        critical: None,
                                    });
                                }
                            }
                            continue;
                        }

                        if fname.starts_with("in") && fname.ends_with("_input")
                            && fname[2..].starts_with(|c: char| c.is_ascii_digit())
                        {
                            let idx = fname.trim_start_matches("in").trim_end_matches("_input");
                            let label = std::fs::read_to_string(
                                    hwmon_path.join(format!("in{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Voltage {}", chip_name, idx));
                            if let Ok(val) = std::fs::read_to_string(hwmon_path.join(fname)) {
                                if let Ok(mv) = val.trim().parse::<f64>() {
                                    let volts = mv / 1000.0;
                                    sensors.push(SensorInfo {
                                        label,
                                        chip: chip_name.clone(),
                                        sensor_type: "in".to_string(),
                                        value: volts,
                                        unit: "V".to_string(),
                                        temp: volts as f32,
                                        max: None,
                                        limit: None,
                                        critical: None,
                                    });
                                }
                            }
                            continue;
                        }

                        if fname.starts_with("power")
                            && (fname.ends_with("_input") || fname.ends_with("_average"))
                        {
                            let idx_end = if fname.ends_with("_input") { "_input" } else { "_average" };
                            let idx = fname.trim_start_matches("power").trim_end_matches(idx_end);
                            let label = std::fs::read_to_string(
                                    hwmon_path.join(format!("power{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Power {}", chip_name, idx));
                            if sensors.iter().any(|s| s.label == label && s.sensor_type == "power") {
                                continue;
                            }
                            if let Ok(val) = std::fs::read_to_string(hwmon_path.join(fname)) {
                                if let Ok(uw) = val.trim().parse::<f64>() {
                                    let watts = uw / 1_000_000.0;
                                    sensors.push(SensorInfo {
                                        label,
                                        chip: chip_name.clone(),
                                        sensor_type: "power".to_string(),
                                        value: watts,
                                        unit: "W".to_string(),
                                        temp: watts as f32,
                                        max: None,
                                        limit: None,
                                        critical: None,
                                    });
                                }
                            }
                            continue;
                        }

                        if fname.starts_with("curr") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("curr").trim_end_matches("_input");
                            let label = std::fs::read_to_string(
                                    hwmon_path.join(format!("curr{}_label", idx)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Current {}", chip_name, idx));
                            if let Ok(val) = std::fs::read_to_string(hwmon_path.join(fname)) {
                                if let Ok(ma) = val.trim().parse::<f64>() {
                                    let amps = ma / 1000.0;
                                    sensors.push(SensorInfo {
                                        label,
                                        chip: chip_name.clone(),
                                        sensor_type: "curr".to_string(),
                                        value: amps,
                                        unit: "A".to_string(),
                                        temp: amps as f32,
                                        max: None,
                                        limit: None,
                                        critical: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let type_order = |t: &str| -> u8 {
            match t {
                "temp" => 0,
                "fan" => 1,
                "in" => 2,
                "power" => 3,
                "curr" => 4,
                _ => 5,
            }
        };
        sensors.sort_by(|a, b| {
            type_order(&a.sensor_type).cmp(&type_order(&b.sensor_type))
                .then_with(|| a.label.cmp(&b.label))
        });
        
        sensors
    }

    pub fn get_global_disk_io(&mut self) -> (u64, u64) {
        let current_stats = self.parse_disk_stats();
        let mut total_read_rate = 0;
        let mut total_write_rate = 0;
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        
        for (dev, curr) in &current_stats {
            if let Some(prev) = self.prev_disk_stats.get(dev) {
                let read_rate = (curr.read_bytes.saturating_sub(prev.read_bytes) as f64 / elapsed) as u64;
                let write_rate = (curr.write_bytes.saturating_sub(prev.write_bytes) as f64 / elapsed) as u64;
                total_read_rate += read_rate;
                total_write_rate += write_rate;
            }
        }
        
        self.prev_disk_stats = current_stats;
        self.last_update = now;
        
        (total_read_rate, total_write_rate)
    }
    
    pub fn calculate_total_network_io(&self, networks: &[DetailedNetInfo]) -> (u64, u64) {
        let total_down = networks.iter().map(|n| n.down_rate).sum();
        let total_up = networks.iter().map(|n| n.up_rate).sum();
        (total_down, total_up)
    }

    pub fn get_process_nice(pid: Pid) -> i32 {
        unsafe { getpriority(0, pid.as_u32()) }
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

    pub fn get_process_io(pid: Pid) -> (u64, u64, u64, u64) {
        let path = format!("/proc/{}/io", pid);
        if let Ok(content) = fs::read_to_string(path) {
            let mut read_bytes = 0;
            let mut write_bytes = 0;
            let mut rchar = 0;
            let mut wchar = 0;
            for line in content.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().parse::<u64>().unwrap_or(0);
                    match k.trim() {
                        "read_bytes" => read_bytes = v,
                        "write_bytes" => write_bytes = v,
                        "rchar" => rchar = v,
                        "wchar" => wchar = v,
                        _ => {}
                    }
                }
            }
            (read_bytes, write_bytes, rchar, wchar)
        } else {
            (0, 0, 0, 0)
        }
    }

    pub fn get_mount_inodes(mount_point: &str) -> (Option<u64>, Option<u64>, Option<u64>) {
        use std::ffi::CString;
        if let Ok(c_path) = CString::new(mount_point) {
            let mut stat = std::mem::MaybeUninit::<StatVfs>::zeroed();
            unsafe {
                if statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
                    let s = stat.assume_init();
                    if s.f_files > 0 {
                        let total = s.f_files;
                        let free = s.f_ffree;
                        let used = total.saturating_sub(free);
                        return (Some(total), Some(free), Some(used));
                    }
                }
            }
        }
        (None, None, None)
    }

    pub fn get_mount_options() -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let mount_point = parts[1].to_string();
                    let options = parts[3].to_string();
                    map.insert(mount_point, options);
                }
            }
        }
        map
    }
}

#[repr(C)]
struct StatVfs {
    f_bsize: std::ffi::c_ulong,
    f_frsize: std::ffi::c_ulong,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: std::ffi::c_ulong,
    f_flag: std::ffi::c_ulong,
    f_namemax: std::ffi::c_ulong,
    __f_spare: [std::ffi::c_int; 6],
}

extern "C" {
    fn statvfs(path: *const std::ffi::c_char, buf: *mut StatVfs) -> std::ffi::c_int;
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
        assert!(monitor.system.cpus().len() > 0);
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
        let pid = sysinfo::Pid::from(std::process::id() as usize);
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
        let (total, free, used) = SystemMonitor::get_mount_inodes("/");
        assert!(total.is_some(), "statvfs on root / should succeed");
        let total = total.unwrap();
        let free = free.unwrap();
        let used = used.unwrap();
        assert!(total > 0, "root filesystem should have total inodes > 0");
        assert!(total >= free, "total inodes must be >= free inodes");
        assert_eq!(used, total.saturating_sub(free));

        let mount_opts = SystemMonitor::get_mount_options();
        assert!(!mount_opts.is_empty(), "mount options map should not be empty");

        let pid = sysinfo::Pid::from(std::process::id() as usize);
        let (read_bytes, _write_bytes, rchar, _wchar) = SystemMonitor::get_process_io(pid);
        assert!(rchar >= read_bytes);
    }
}