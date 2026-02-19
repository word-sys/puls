use std::collections::HashMap;
use std::time::Instant;
use sysinfo::{DiskUsage, Networks, Pid, System, Components};
use users::{Users, UsersCache};
use chrono::prelude::*;

use crate::types::*;
use crate::utils::*;

pub struct SystemMonitor {
    system: System,
    components: Components,
    users_cache: UsersCache,
    prev_disk_usage: HashMap<Pid, DiskUsage>,
    prev_net_usage: HashMap<String, NetworkStats>,
    last_update: Instant,
    self_pid: u32,
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
            last_update: Instant::now(),
            self_pid: std::process::id(),
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        vec![
            ("OS".into(), System::long_os_version().unwrap_or_default()),
            ("Kernel".into(), System::kernel_version().unwrap_or_default()),
            ("Hostname".into(), System::host_name().unwrap_or_default()),
            ("CPU".into(), self.system.cpus().get(0).map_or("N/A".into(), |c| c.brand().to_string())),
            ("Cores".into(), format!("{} Physical / {} Logical", 
                self.system.physical_core_count().unwrap_or(0), 
                self.system.cpus().len())),
            ("Total Memory".into(), format_size(self.system.total_memory())),
            ("Boot Time".into(), {
                let boot_time = System::boot_time(); if boot_time > 0 {
                    if let chrono::LocalResult::Single(dt) = Utc.timestamp_opt(boot_time as i64, 0) {
                        dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
                    } else {
                        "Unknown".to_string()
                    }
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
        ]
    }

    pub fn get_total_memory(&self) -> u64 {
        self.system.total_memory()
    }
    
    pub fn update_processes(&mut self, show_system: bool, filter: &str) -> Vec<ProcessInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.components.refresh(true);
        
        let total_cpu_count = self.system.cpus().len() as f32;
        let mut current_disk_usage = HashMap::new();
        let processes: Vec<ProcessInfo> = self.system.processes()
            .iter()
            .filter(|(_pid, process)| {
                /*
                if pid.as_u32() == self.self_pid {
                    return false;
                }
                */
                
                if !show_system && is_system_process(&process.name().to_string_lossy()) {
                    return false;
                }
                
                if !filter.is_empty() {
                    let search_text = format!("{} {}", process.name().to_string_lossy(), process.pid());
                    if !matches_filter(&search_text, filter) {
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
                    .map_or("N/A".to_string(), |u| u.name().to_string_lossy().into_owned());
                
                let raw_cpu = process.cpu_usage();
                let normalized_cpu = (raw_cpu / total_cpu_count).clamp(0.0, 100.0);
                
                let mut status = process.status().to_string();
                
                if pid.as_u32() == self.self_pid || normalized_cpu > 0.0 {
                     status = "Running".to_string();
                }

                ProcessInfo {
                    pid: pid.to_string(),
                    name: process.name().to_string_lossy().to_string(),
                    cpu: normalized_cpu,
                    cpu_display: format!("{:.2}%", normalized_cpu),
                    mem: process.memory(),
                    mem_display: format_size(process.memory()),
                    disk_read: format_rate(read_rate),
                    disk_write: format_rate(write_rate),
                    user,
                    status,
                }
            })
            .collect();
        
        self.prev_disk_usage = current_disk_usage;
        processes
    }
    
    pub fn get_detailed_process(&self, pid: Pid) -> Option<DetailedProcessInfo> {
        self.system.process(pid).map(|process| {
            let start_time = if let chrono::LocalResult::Single(dt) = 
                Utc.timestamp_opt(process.start_time() as i64, 0) {
                dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "Invalid time".to_string()
            };
            
            let user = process.user_id()
                .and_then(|uid| self.users_cache.get_user_by_uid(**uid))
                .map_or("N/A".to_string(), |u| u.name().to_string_lossy().into_owned());
            
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
                environ: process.environ().iter().map(|s| s.to_string_lossy().to_string()).collect(),
                threads: process.tasks().map(|t| t.len() as u32).unwrap_or(0),
                file_descriptors: None,
                cwd: process.cwd().map(|p| p.to_string_lossy().into_owned()),
            }
        })
    }
    
    pub fn get_cores(&self) -> Vec<CoreInfo> {
        let components = &self.components;
        // k10temp coretemp zenpower
        
        let cpu_temp_global = components.iter()
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
            .or_else(|| Self::read_cpu_temp_from_hwmon());

        self.system.cpus().iter().enumerate().map(|(i, cpu)| {
            let core_temp = components.iter()
                .find(|c| c.label().to_lowercase().contains(&format!("core {}", i)))
                .and_then(|c| c.temperature())
                .or(cpu_temp_global);

            CoreInfo {
                usage: cpu.cpu_usage(),
                freq: cpu.frequency(),
                temp: core_temp,
            }
        }).collect()
    }
    
    pub fn get_disks(&self) -> Vec<DetailedDiskInfo> {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        disks.iter().map(|disk| {
            let used = disk.total_space().saturating_sub(disk.available_space());
            let disk_name = disk.name().to_string_lossy();
            
            let temp = self.components.iter()
                .find(|c| {
                    let label = c.label().to_string();
                    label.contains(disk_name.as_ref()) || disk_name.contains(label.as_str())
                })
                .and_then(|c| c.temperature())
                .or_else(|| {
                    let dev_str_fb = disk_name.to_string();
                    let block_dev_fb = dev_str_fb.split('/').last().unwrap_or(&dev_str_fb);
                    let base_fb = if block_dev_fb.contains("nvme") {
                        block_dev_fb.rfind('p')
                            .and_then(|pos| {
                                if pos > 0 && block_dev_fb[pos+1..].chars().all(|c| c.is_ascii_digit()) {
                                    Some(&block_dev_fb[..pos])
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(block_dev_fb)
                    } else {
                        block_dev_fb.trim_end_matches(|c: char| c.is_ascii_digit())
                    };
                    // /sys/block/<dev>/device/hwmon/hwmon*/temp1_input
                    let hwmon_path = format!("/sys/block/{}/device/hwmon", base_fb);
                    std::fs::read_dir(&hwmon_path).ok().and_then(|entries| {
                        for entry in entries.flatten() {
                            let temp_file = entry.path().join("temp1_input");
                            if let Ok(val) = std::fs::read_to_string(&temp_file) {
                                if let Ok(millideg) = val.trim().parse::<f32>() {
                                    return Some(millideg / 1000.0);
                                }
                            }
                        }
                        None
                    })
                });
            
            let dev_str = disk_name.to_string();
            let block_dev = dev_str.split('/').last().unwrap_or(&dev_str);
            let base_dev = if block_dev.contains("nvme") {
                block_dev.rfind('p')
                    .and_then(|pos| {
                        if pos > 0 && block_dev[pos+1..].chars().all(|c| c.is_ascii_digit()) {
                            Some(&block_dev[..pos])
                        } else {
                            None
                        }
                    })
                    .unwrap_or(block_dev)
            } else {
                block_dev.trim_end_matches(|c: char| c.is_ascii_digit())
            };

            let mut health_pct: Option<u8> = None;
            let mut power_cycles: Option<u64> = None;
            let is_nvme = base_dev.starts_with("nvme");
            
            if is_nvme {
                // /sys/block/nvme0n1/device/percentage_used
                if let Ok(val) = std::fs::read_to_string(format!("/sys/block/{}/device/percentage_used", base_dev)) {
                    if let Ok(pct) = val.trim().parse::<u8>() {
                        health_pct = Some(100u8.saturating_sub(pct));
                    }
                }
                // /sys/block/nvme0n1/device/power_cycles  
                if let Ok(val) = std::fs::read_to_string(format!("/sys/block/{}/device/power_cycles", base_dev)) {
                    power_cycles = val.trim().parse::<u64>().ok();
                }
            }

            let is_ssd = std::fs::read_to_string(format!("/sys/block/{}/queue/rotational", base_dev))
                .ok()
                .and_then(|v| v.trim().parse::<u8>().ok())
                .map(|v| v == 0);
            
            DetailedDiskInfo {
                name: disk.mount_point().to_string_lossy().into_owned(),
                device: disk.name().to_string_lossy().into_owned(),
                fs: disk.file_system().to_string_lossy().to_string(),
                total: disk.total_space(),
                free: disk.available_space(),
                used,
                read_rate: 0,
                write_rate: 0,
                read_ops: 0,
                write_ops: 0,
                is_ssd,
                temp,
                health_pct,
                power_cycles,
            }
        }).collect()
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
    
    pub fn get_global_usage(&self, total_net_down: u64, total_net_up: u64, 
                           total_disk_read: u64, total_disk_write: u64,
                           gpu_util: Option<u32>) -> GlobalUsage {
        let load = System::load_average();
        let boot_time = System::boot_time();
        let uptime = current_timestamp().saturating_sub(boot_time);
        
        let mem_available = self.system.available_memory();
        let mem_free = self.system.free_memory();
        let mem_cached = mem_available.saturating_sub(mem_free);

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
            disk_read_ops: 0, //will be
            disk_write_ops: 0, //done 
            load_average: (load.one, load.five, load.fifteen),
            uptime,
            boot_time,
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

    pub fn get_sensors(&self) -> Vec<SensorInfo> {
        let mut sensors = Vec::new();
        
        for c in self.components.iter() {
            sensors.push(SensorInfo {
                label: c.label().to_string(),
                chip: String::new(),
                sensor_type: "temp".to_string(),
                value: c.temperature().unwrap_or(0.0) as f64,
                unit: "°C".to_string(),
                temp: c.temperature().unwrap_or(0.0),
                max: c.max(),
                critical: c.critical(),
            });
        }
        
        if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                let chip_name = std::fs::read_to_string(hwmon_path.join("name"))
                    .unwrap_or_default().trim().to_string();
                if let Ok(files) = std::fs::read_dir(&hwmon_path) {
                    for file in files.flatten() {
                        let fname = file.file_name().to_string_lossy().to_string();
                        if fname.starts_with("fan") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("fan").trim_end_matches("_input");
                            let label_file = hwmon_path.join(format!("fan{}_label", idx));
                            let label = std::fs::read_to_string(&label_file)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Fan {}", chip_name, idx));
                            if let Ok(val) = std::fs::read_to_string(file.path()) {
                                if let Ok(rpm) = val.trim().parse::<f64>() {
                                    sensors.push(SensorInfo {
                                        label,
                                        chip: chip_name.clone(),
                                        sensor_type: "fan".to_string(),
                                        value: rpm,
                                        unit: "RPM".to_string(),
                                        temp: rpm as f32,
                                        max: None,
                                        critical: None,
                                    });
                                }
                            }
                        }
                        
                        if fname.starts_with("in") && fname.ends_with("_input") && fname[2..].starts_with(|c: char| c.is_ascii_digit()) {
                            let idx = fname.trim_start_matches("in").trim_end_matches("_input");
                            let label_file = hwmon_path.join(format!("in{}_label", idx));
                            let label = std::fs::read_to_string(&label_file)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Voltage {}", chip_name, idx));
                            
                            if let Ok(val) = std::fs::read_to_string(file.path()) {
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
                                        critical: None,
                                    });
                                }
                            }
                        }
                        
                        if fname.starts_with("power") && (fname.ends_with("_input") || fname.ends_with("_average")) {
                            let idx_end = if fname.ends_with("_input") { "_input" } else { "_average" };
                            let idx = fname.trim_start_matches("power").trim_end_matches(idx_end);
                            let label_file = hwmon_path.join(format!("power{}_label", idx));
                            let label = std::fs::read_to_string(&label_file)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Power {}", chip_name, idx));
                            
                            if sensors.iter().any(|s| s.label == label && s.sensor_type == "power") {
                                continue;
                            }
                            
                            if let Ok(val) = std::fs::read_to_string(file.path()) {
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
                                        critical: None,
                                    });
                                }
                            }
                        }
                        
                        if fname.starts_with("curr") && fname.ends_with("_input") {
                            let idx = fname.trim_start_matches("curr").trim_end_matches("_input");
                            let label_file = hwmon_path.join(format!("curr{}_label", idx));
                            let label = std::fs::read_to_string(&label_file)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{} Current {}", chip_name, idx));
                            
                            if let Ok(val) = std::fs::read_to_string(file.path()) {
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
    
    pub fn refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.components.refresh(true);
    }
    
    pub fn calculate_total_disk_io(&self, processes: &[ProcessInfo]) -> (u64, u64) {
        let total_read = processes.iter()
            .map(|p| p.disk_read.trim_end_matches(" B/s").trim_end_matches(" KB/s").trim_end_matches(" MB/s")
                .parse::<f64>().unwrap_or(0.0) as u64)
            .sum();
        let total_write = processes.iter()
            .map(|p| p.disk_write.trim_end_matches(" B/s").trim_end_matches(" KB/s").trim_end_matches(" MB/s")
                .parse::<f64>().unwrap_or(0.0) as u64)
            .sum();
        
        (total_read, total_write)
    }
    
    pub fn calculate_total_network_io(&self, networks: &[DetailedNetInfo]) -> (u64, u64) {
        let total_down = networks.iter().map(|n| n.down_rate).sum();
        let total_up = networks.iter().map(|n| n.up_rate).sum();
        (total_down, total_up)
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sort_processes(processes: &mut Vec<ProcessInfo>, sort_by: &ProcessSortBy, ascending: bool, total_memory: u64) {
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
        ProcessSortBy::Pid => {
            processes.sort_by(|a, b| {
                let a_pid: u32 = a.pid.parse().unwrap_or(0);
                let b_pid: u32 = b.pid.parse().unwrap_or(0);
                let cmp = a_pid.cmp(&b_pid);
                if ascending { cmp } else { cmp.reverse() }
            });
        },
        ProcessSortBy::DiskRead | ProcessSortBy::DiskWrite => {
            processes.sort_by(|a, b| {
                let cmp = a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal);
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
                cpu_display: "1.0%".to_string(),
                mem: 1024,
                mem_display: "1.0 KiB".to_string(),
                disk_read: "0 B/s".to_string(),
                disk_write: "0 B/s".to_string(),
                user: "root".to_string(),
                status: "Running".to_string(),
            },
            ProcessInfo {
                pid: "2".to_string(),
                name: "kthreadd".to_string(),
                cpu: 5.0,
                cpu_display: "5.0%".to_string(),
                mem: 2048,
                mem_display: "2.0 KiB".to_string(),
                disk_read: "0 B/s".to_string(),
                disk_write: "0 B/s".to_string(),
                user: "root".to_string(),
                status: "Running".to_string(),
            },
        ];
        
        sort_processes(&mut processes, &ProcessSortBy::Cpu, false, 8192 * 1024 * 1024);
        assert_eq!(processes[0].name, "kthreadd");
        
        sort_processes(&mut processes, &ProcessSortBy::Memory, false, 8192 * 1024 * 1024);
        assert_eq!(processes[0].name, "kthreadd");
    }
}