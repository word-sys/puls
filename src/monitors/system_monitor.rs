use std::collections::HashMap;
use std::time::Instant;
use sysinfo::{CpuExt, DiskExt, DiskUsage, NetworkExt, NetworksExt, Pid, PidExt, ProcessExt, System, SystemExt};
use users::UsersCache;
use chrono::prelude::*;

use crate::types::*;
use crate::utils::*;

pub struct SystemMonitor {
    system: System,
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
        
        Self {
            system,
            users_cache: UsersCache::new(),
            prev_disk_usage: HashMap::new(),
            prev_net_usage: HashMap::new(),
            last_update: Instant::now(),
            self_pid: std::process::id(),
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        vec![
            ("OS".into(), self.system.long_os_version().unwrap_or_default()),
            ("Kernel".into(), self.system.kernel_version().unwrap_or_default()),
            ("Hostname".into(), self.system.host_name().unwrap_or_default()),
            ("CPU".into(), self.system.cpus().get(0).map_or("N/A".into(), |c| c.brand().to_string())),
            ("Cores".into(), format!("{} Physical / {} Logical", 
                self.system.physical_core_count().unwrap_or(0), 
                self.system.cpus().len())),
            ("Total Memory".into(), format_size(self.system.total_memory())),
            ("Boot Time".into(), {
                if let Some(boot_time) = self.system.boot_time() {
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
                if let Some(boot_time) = self.system.boot_time() {
                    let uptime = current_timestamp().saturating_sub(boot_time);
                    format_duration(uptime)
                } else {
                    "Unknown".to_string()
                }
            }),
            ("Load Average".into(), {
                let load = self.system.load_average();
                format!("{:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen)
            }),
        ]
    }
    
    /// Update system information and get processes
    pub fn update_processes(&mut self, show_system: bool, filter: &str) -> Vec<ProcessInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;
        
        self.system.refresh_all();
        
        let mut current_disk_usage = HashMap::new();
        let mut processes: Vec<ProcessInfo> = self.system.processes()
            .iter()
            .filter(|(pid, process)| {
                // Filter out self
                if pid.as_u32() == self.self_pid {
                    return false;
                }
                
                // Filter system processes if needed
                if !show_system && is_system_process(process.name()) {
                    return false;
                }
                
                // Apply text filter
                if !filter.is_empty() {
                    let search_text = format!("{} {}", process.name(), process.pid());
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
                
                ProcessInfo {
                    pid: pid.to_string(),
                    name: process.name().to_string(),
                    cpu: process.cpu_usage(),
                    cpu_display: format!("{:.2}%", process.cpu_usage()),
                    mem: process.memory(),
                    mem_display: format_size(process.memory()),
                    disk_read: format_rate(read_rate),
                    disk_write: format_rate(write_rate),
                    user,
                    status: process.status().to_string(),
                }
            })
            .collect();
        
        self.prev_disk_usage = current_disk_usage;
        processes
    }
    
    /// Get detailed information for a specific process
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
                name: process.name().to_string(),
                user,
                status: process.status().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_rss: process.memory(),
                memory_vms: process.virtual_memory(),
                command: process.cmd().join(" "),
                start_time,
                parent: process.parent().map(|p| p.to_string()),
                environ: process.environ().to_vec(),
                threads: process.tasks().map(|t| t.len() as u32).unwrap_or(0),
                file_descriptors: None, // TODO: Implement if available
                cwd: process.cwd().map(|p| p.to_string_lossy().into_owned()),
            }
        })
    }
    
    /// Get CPU core information
    pub fn get_cores(&self) -> Vec<CoreInfo> {
        self.system.cpus().iter().map(|cpu| CoreInfo {
            usage: cpu.cpu_usage(),
            freq: cpu.frequency(),
            temp: None, // TODO: Implement temperature reading per core
        }).collect()
    }
    
    /// Get disk information with I/O rates
    pub fn get_disks(&self) -> Vec<DetailedDiskInfo> {
        self.system.disks().iter().map(|disk| {
            let used = disk.total_space().saturating_sub(disk.available_space());
            
            DetailedDiskInfo {
                name: disk.mount_point().to_string_lossy().into_owned(),
                device: disk.name().to_string_lossy().into_owned(),
                fs: String::from_utf8_lossy(disk.file_system()).into_owned(),
                total: disk.total_space(),
                free: disk.available_space(),
                used,
                read_rate: 0,  // TODO: Implement disk I/O rates
                write_rate: 0,
                read_ops: 0,
                write_ops: 0,
                is_ssd: None, // TODO: Detect SSD vs HDD
            }
        }).collect()
    }
    
    /// Get network interface information with rates
    pub fn get_networks(&mut self) -> Vec<DetailedNetInfo> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        
        let mut current_net_usage = HashMap::new();
        let networks: Vec<DetailedNetInfo> = self.system.networks()
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
                    interface_type: "Unknown".to_string(), // TODO: Detect interface type
                    is_up: true, // TODO: Detect interface status
                }
            })
            .collect();
        
        self.prev_net_usage = current_net_usage;
        networks
    }
    
    /// Get global system usage statistics
    pub fn get_global_usage(&self, total_net_down: u64, total_net_up: u64, 
                           total_disk_read: u64, total_disk_write: u64,
                           gpu_util: Option<u32>) -> GlobalUsage {
        let load = self.system.load_average();
        let boot_time = self.system.boot_time().unwrap_or(0);
        let uptime = current_timestamp().saturating_sub(boot_time);
        
        GlobalUsage {
            cpu: self.system.global_cpu_info().cpu_usage(),
            mem_used: self.system.used_memory(),
            mem_total: self.system.total_memory(),
            gpu_util,
            net_down: total_net_down,
            net_up: total_net_up,
            disk_read: total_disk_read,
            disk_write: total_disk_write,
            load_average: (load.one, load.five, load.fifteen),
            uptime,
            boot_time,
            ..Default::default() // Will be updated with history in the caller
        }
    }
    
    /// Get system temperatures (if available)
    pub fn get_temperatures(&self) -> SystemTemperatures {
        SystemTemperatures {
            cpu_temp: None, // TODO: Implement CPU temperature reading
            gpu_temps: Vec::new(), // Will be filled by GPU monitor
            motherboard_temp: None, // TODO: Implement motherboard temperature
        }
    }
    
    /// Refresh system information
    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }
    
    /// Get total disk I/O from all processes
    pub fn calculate_total_disk_io(&self, processes: &[ProcessInfo]) -> (u64, u64) {
        // This is a simple approximation - in reality we'd need to track system-wide I/O
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

pub fn sort_processes(processes: &mut Vec<ProcessInfo>, sort_by: &ProcessSortBy, ascending: bool) {
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
        
        sort_processes(&mut processes, &ProcessSortBy::Cpu, false);
        assert_eq!(processes[0].name, "kthreadd");
        
        sort_processes(&mut processes, &ProcessSortBy::Memory, false);
        assert_eq!(processes[0].name, "kthreadd");
    }
}