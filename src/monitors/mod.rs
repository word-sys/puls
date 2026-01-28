#![allow(dead_code)]

pub mod system_monitor;
pub mod gpu_monitor;
pub mod container_monitor;

pub use system_monitor::SystemMonitor;
pub use gpu_monitor::GpuMonitor;
pub use container_monitor::ContainerMonitor;

use std::sync::Arc;
use parking_lot::Mutex;
use tokio::time::{Duration, Instant};

use crate::types::{DynamicData, AppConfig, GlobalUsage};
use crate::utils::update_history;

pub struct DataCollector {
    system_monitor: SystemMonitor,
    gpu_monitor: GpuMonitor,
    container_monitor: ContainerMonitor,
    config: AppConfig,
    last_update: Instant,
}

impl DataCollector {
    pub fn new(config: AppConfig) -> Self {
        Self {
            system_monitor: SystemMonitor::new(),
            gpu_monitor: GpuMonitor::new(),
            container_monitor: ContainerMonitor::new(),
            config,
            last_update: Instant::now(),
        }
    }
    
    pub async fn collect_data(
        &mut self,
        selected_pid: Option<sysinfo::Pid>,
        show_system_processes: bool,
        filter: &str,
        sort_by: &crate::types::ProcessSortBy,
        sort_ascending: bool,
        mut prev_global_usage: GlobalUsage,
    ) -> DynamicData {
        let now = Instant::now();
        let collection_start = now;
        let mut processes = self.system_monitor.update_processes(
            show_system_processes,
            filter
        );
        
        crate::monitors::system_monitor::sort_processes(
            &mut processes,
            sort_by,
            sort_ascending,
            self.system_monitor.get_total_memory()
        );    
 
        let detailed_process = selected_pid
            .and_then(|pid| self.system_monitor.get_detailed_process(pid));
        
        let cores = self.system_monitor.get_cores();
        
        let disks = self.system_monitor.get_disks();
        
        let networks = if self.config.enable_network_monitoring {
            self.system_monitor.get_networks()
        } else {
            Vec::new()
        };
        
        let (total_net_down, total_net_up) = self.system_monitor
            .calculate_total_network_io(&networks);
        
        let (total_disk_read, total_disk_write) = self.system_monitor
            .calculate_total_disk_io(&processes);
        
        let (containers, docker_error) = if self.config.enable_docker && self.container_monitor.is_available() {
            match tokio::time::timeout(
                self.config.get_operation_timeout(),
                self.container_monitor.get_containers(self.config.get_operation_timeout().as_millis() as u64)
            ).await {
                Ok(Ok(containers)) => (containers, None),
                Ok(Err(e)) => (Vec::new(), Some(e)),
                Err(_) => (Vec::new(), Some("Container collection timeout".to_string())),
            }
        } else {
            (Vec::new(), None)
        };
        
        let gpus = if !self.config.enable_gpu_monitoring {
            Err("GPU monitoring disabled by configuration".to_string())
        } else if !self.gpu_monitor.is_available() {
            Err("GPU monitoring unavailable (monitor reports not available)".to_string())
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
        
        let temperatures = self.system_monitor.get_temperatures();
        
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
        
        let collection_end = Instant::now();
        let collection_duration = collection_end.duration_since(collection_start);
        
        if collection_duration > Duration::from_millis(self.config.refresh_rate_ms / 2) {
            eprintln!("Slow data collection: {:?}", collection_duration);
        }
        
        DynamicData {
            processes,
            detailed_process,
            cores,
            disks,
            networks,
            containers,
            gpus,
            global_usage,
            temperatures,
            last_update: std::time::Instant::now(),
            docker_error,
        }
    }
    
    pub fn get_system_info(&self) -> Vec<(String, String)> {
        let mut info = self.system_monitor.get_system_info();
        
        if self.config.safe_mode {
            info.push(("Mode".to_string(), "Safe Mode".to_string()));
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
    
    pub async fn health_check(&self) -> Vec<(String, bool)> {
        let mut health = Vec::new();
        
        health.push(("System".to_string(), true));
        
        if self.config.enable_docker {
            let docker_health = self.container_monitor.health_check(1000).await;
            health.push(("Docker".to_string(), docker_health));
        }
        
        if self.config.enable_gpu_monitoring {
            health.push(("GPU".to_string(), self.gpu_monitor.is_available()));
        }
        
        if self.config.enable_network_monitoring {
            health.push(("Network".to_string(), true));
        }
        
        health
    }
}

pub type SharedDataCollector = Arc<Mutex<DataCollector>>;