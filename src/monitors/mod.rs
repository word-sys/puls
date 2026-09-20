pub mod system_monitor;
pub mod gpu_monitor;
pub mod container_monitor;
pub mod network_sockets;
pub mod power_monitor;
pub mod numa_monitor;

pub use system_monitor::SystemMonitor;
pub use gpu_monitor::GpuMonitor;
pub use container_monitor::ContainerMonitor;
pub use power_monitor::PowerMonitor;
pub use numa_monitor::NumaMonitor;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::types::{DynamicData, AppConfig, GlobalUsage};
use crate::utils::update_history;

pub struct DataCollector {
    system_monitor: SystemMonitor,
    gpu_monitor: GpuMonitor,
    container_monitor: ContainerMonitor,
    power_monitor: PowerMonitor,
    numa_monitor: NumaMonitor,
    config: AppConfig,
}

impl DataCollector {
    pub fn new(config: AppConfig) -> Self {
        Self {
            system_monitor: SystemMonitor::new(),
            gpu_monitor: GpuMonitor::new(),
            container_monitor: ContainerMonitor::new(),
            power_monitor: PowerMonitor::new(),
            numa_monitor: NumaMonitor::new(),
            config,
        }
    }
    
    #[allow(clippy::too_many_arguments)]
    pub async fn collect_data(
        &mut self,
        selected_pid: Option<sysinfo::Pid>,
        show_system_processes: bool,
        filter: &str,
        sort_by: &crate::types::ProcessSortBy,
        sort_ascending: bool,
        tree_mode: bool,
        mut prev_global_usage: GlobalUsage,
        active_tab: usize,
    ) -> DynamicData {
        self.system_monitor.refresh_core_metrics();
        
        let processes = if active_tab == 0 || active_tab == 1 || active_tab == 7 {
            let mut procs = self.system_monitor.update_processes(show_system_processes, filter);
            if tree_mode && active_tab == 1 {
                crate::monitors::system_monitor::build_process_tree(
                    &mut procs,
                    sort_by,
                    sort_ascending,
                    self.system_monitor.get_total_memory(),
                );
            } else {
                crate::monitors::system_monitor::sort_processes(
                    &mut procs,
                    sort_by,
                    sort_ascending,
                    self.system_monitor.get_total_memory(),
                );
            }
            procs
        } else {
            Vec::new()
        };

        let detailed_process = if active_tab == 1 && selected_pid.is_some() {
            selected_pid.and_then(|pid| self.system_monitor.get_detailed_process(pid))
        } else {
            None
        };
        
        let cores = if active_tab == 1 || active_tab == 2 {
            self.system_monitor.get_cores()
        } else {
            Vec::new()
        };
        
        let disks = if active_tab == 4 || active_tab == 0 {
            self.system_monitor.get_disks()
        } else {
            Vec::new()
        };
        
        let networks = if self.config.enable_network_monitoring {
            self.system_monitor.get_networks()
        } else {
            Vec::new()
        };

        let sockets = if active_tab == 5 && self.config.enable_network_monitoring {
            network_sockets::get_active_sockets()
        } else {
            Vec::new()
        };
        
        let (total_net_down, total_net_up) = self.system_monitor
            .calculate_total_network_io(&networks);
        
        let (total_disk_read, total_disk_write) = self.system_monitor
            .get_global_disk_io();

        let (containers, docker_error) = if self.config.enable_docker && (active_tab == 11 || active_tab == 0) {
            if self.container_monitor.is_available() {
                match tokio::time::timeout(
                    self.config.get_operation_timeout(),
                    self.container_monitor.get_containers(self.config.get_operation_timeout().as_millis() as u64)
                ).await {
                    Ok(Ok(containers)) => (containers, None),
                    Ok(Err(e)) => (Vec::new(), Some(e)),
                    Err(_) => (Vec::new(), Some("Container collection timeout".to_string())),
                }
            } else {
                #[cfg(feature = "docker")]
                { (Vec::new(), self.container_monitor.init_error.clone()) }
                #[cfg(not(feature = "docker"))]
                { (Vec::new(), None) }
            }
        } else {
            (Vec::new(), None)
        };
        
        let gpus = if !self.config.enable_gpu_monitoring {
            Err("GPU monitoring disabled by configuration".to_string())
        } else if !self.gpu_monitor.is_available() {
            Err("GPU monitoring unavailable".to_string())
        } else {
            self.gpu_monitor.get_gpu_info()
        };
        
        let gpu_util = match &gpus {
            Ok(gpu_list) => self.gpu_monitor.get_primary_gpu_utilization(gpu_list),
            Err(_) => None,
        };
        
        if let Ok(ref gpu_list) = gpus {
            self.gpu_monitor.update_gpu_history(gpu_list, self.config.history_length);
        }
        
        let (temperatures, sensors) = (self.system_monitor.get_temperatures(), self.system_monitor.get_sensors());

        let networks = if active_tab == 5 || active_tab == 0 {
            networks
        } else {
            Vec::new()
        };

        let gpus = if active_tab == 6 || active_tab == 0 {
            gpus
        } else {
            Err("Not on GPU tab".to_string())
        };
        
        let mut global_usage = self.system_monitor.get_global_usage(
            total_net_down,
            total_net_up,
            total_disk_read,
            total_disk_write,
            gpu_util,
        );
        
        update_history(&mut prev_global_usage.cpu_history, global_usage.cpu, self.config.history_length);
        update_history(&mut prev_global_usage.mem_history, 
            (global_usage.mem_used as f64 / global_usage.mem_total as f64 * 100.0) as f32, 
            self.config.history_length);
        update_history(&mut prev_global_usage.net_down_history, total_net_down, self.config.history_length);
        update_history(&mut prev_global_usage.net_up_history, total_net_up, self.config.history_length);
        update_history(&mut prev_global_usage.disk_read_history, total_disk_read, self.config.history_length);
        update_history(&mut prev_global_usage.disk_write_history, total_disk_write, self.config.history_length);
        
        if let Some(gpu_util_val) = gpu_util {
            update_history(&mut prev_global_usage.gpu_history, gpu_util_val, self.config.history_length);
        }
        
        global_usage.cpu_history = prev_global_usage.cpu_history;
        global_usage.mem_history = prev_global_usage.mem_history;
        global_usage.net_down_history = prev_global_usage.net_down_history;
        global_usage.net_up_history = prev_global_usage.net_up_history;
        global_usage.disk_read_history = prev_global_usage.disk_read_history;
        global_usage.disk_write_history = prev_global_usage.disk_write_history;
        global_usage.gpu_history = prev_global_usage.gpu_history;

        let battery = self.power_monitor.get_battery_info();
        let reboot_required = std::path::Path::new("/var/run/reboot-required").exists()
            || std::path::Path::new("/run/reboot-required").exists();
        let numa_nodes = self.numa_monitor.get_numa_nodes();

        DynamicData {
            processes,
            detailed_process,
            cores,
            disks,
            networks,
            sockets,
            containers,
            gpus,
            global_usage,
            temperatures,
            sensors,
            battery,
            reboot_required,
            numa_nodes,
            last_update: std::time::Instant::now(),
            docker_error,
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        let mut info = self.system_monitor.get_system_info();
        
        if self.config.safe_mode {
            info.push(("Mode".to_string(), "Safe Mode".to_string()));
        }
        
        if let Some(ref bat) = self.power_monitor.get_battery_info() {
            if let Some(ref gov) = bat.cpu_governor {
                let driver_str = bat.cpu_driver.as_deref().unwrap_or("unknown");
                info.push(("CPU Governor".to_string(), format!("{} ({})", gov, driver_str)));
            }
            let ac_str = if bat.ac_online { "Connected" } else { "Disconnected" };
            info.push(("Power Source".to_string(), format!("Battery {} (AC: {})", bat.name, ac_str)));
        }

        let numa_nodes = self.numa_monitor.get_numa_nodes();
        if !numa_nodes.is_empty() {
            info.push(("NUMA Nodes".to_string(), format!("{} Node(s)", numa_nodes.len())));
            for n in &numa_nodes {
                let mem_str = format!("{} / {}", crate::utils::format_size(n.mem_used_bytes), crate::utils::format_size(n.mem_total_bytes));
                let cpus_summary = if n.cpu_list_str.is_empty() {
                    format!("{} cores", n.cpus.len())
                } else {
                    format!("cores {} ({} total)", n.cpu_list_str, n.cpus.len())
                };
                let hit_str = if let (Some(h), Some(m)) = (n.numa_hit, n.numa_miss) {
                    let total = h + m;
                    if total > 0 {
                        format!(" | Hits: {:.1}%", (h as f64 / total as f64) * 100.0)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                info.push((format!("  └─ Node {}", n.id), format!("{} | RAM: {}{}", cpus_summary, mem_str, hit_str)));
            }
        }

        let mut features = Vec::new();
        if self.config.enable_docker && self.container_monitor.is_available() {
            features.push("Docker");
        }
        if self.config.enable_gpu_monitoring && self.gpu_monitor.is_available() {
            features.push("GPU");
        }
        if self.config.enable_network_monitoring {
            features.push("Network");
        }
        
        if !features.is_empty() {
            info.push(("Features".to_string(), features.join(", ")));
        }
        
        info
    }

    pub async fn start_container(&self, id: &str) -> Result<(), String> {
        self.container_monitor.start_container(id).await
    }

    pub async fn stop_container(&self, id: &str) -> Result<(), String> {
        self.container_monitor.stop_container(id).await
    }

    pub async fn restart_container(&self, id: &str) -> Result<(), String> {
        self.container_monitor.restart_container(id).await
    }

    pub async fn pause_container(&self, id: &str) -> Result<(), String> {
        self.container_monitor.pause_container(id).await
    }

    pub async fn unpause_container(&self, id: &str) -> Result<(), String> {
        self.container_monitor.unpause_container(id).await
    }

    pub async fn get_container_logs(&self, id: &str) -> Result<Vec<String>, String> {
        self.container_monitor.get_container_logs(id).await
    }
}

pub type SharedDataCollector = Arc<Mutex<DataCollector>>;