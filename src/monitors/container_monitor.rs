use std::collections::HashMap;
use std::time::{Duration, Instant};
use futures_util::{future, stream::StreamExt};
use tokio::time::timeout;

#[cfg(feature = "docker")]
use bollard::{container::StatsOptions, Docker};

use crate::types::{ContainerInfo, ContainerIoStats};
use crate::utils::{format_size, format_rate, calculate_rate};

pub struct ContainerMonitor {
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    
    prev_container_stats: HashMap<String, ContainerIoStats>,
    last_update: Instant,
}

impl ContainerMonitor {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "docker")]
            docker: Self::init_docker(),
            
            prev_container_stats: HashMap::new(),
            last_update: Instant::now(),
        }
    }
    
    #[cfg(feature = "docker")]
    fn init_docker() -> Option<Docker> {
        match Docker::connect_with_local_defaults() {
            Ok(docker) => Some(docker),
            Err(e) => {
                eprintln!("Failed to connect to Docker: {}", e);
                None
            }
        }
    }
    
    #[cfg(not(feature = "docker"))]
    fn init_docker() -> Option<()> {
        None
    }
    
    pub async fn get_containers(&mut self, timeout_ms: u64) -> Vec<ContainerInfo> {
        #[cfg(feature = "docker")]
        if let Some(ref docker) = self.docker {
            let docker_clone = docker.clone();
            match self.get_docker_containers(&docker_clone, timeout_ms).await {
                Ok(containers) => return containers,
                Err(e) => {
                    eprintln!("Docker error: {}", e);
                    return Vec::new();
                }
            }
        }
        
        // TODO: Add Podman support here
        
        Vec::new()
    }
    
    #[cfg(feature = "docker")]
    async fn get_docker_containers(&mut self, docker: &Docker, timeout_ms: u64) -> Result<Vec<ContainerInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;
        
        if timeout(Duration::from_millis(timeout_ms / 4), docker.ping()).await.is_err() {
            return Err("Docker daemon not accessible".into());
        }
        
        let containers_list = timeout(
            Duration::from_millis(timeout_ms / 2),
            docker.list_containers::<String>(None)
        ).await??;
        
        if containers_list.is_empty() {
            return Ok(Vec::new());
        }
        
        let stats_futures = containers_list.iter()
            .filter_map(|container| container.id.as_ref())
            .map(|id| {
                let docker_clone = docker.clone();
                let id_clone = id.clone();
                let timeout_duration = Duration::from_millis(timeout_ms / 4);
                
                async move {
                    let options = StatsOptions { 
                        stream: false, 
                        ..Default::default() 
                    };
                    
                    let mut stats_stream = docker_clone.stats(&id_clone, Some(options));
                    let result = timeout(timeout_duration, stats_stream.next()).await;
                    
                    (id_clone, result)
                }
            });
        
        let stats_results = future::join_all(stats_futures).await;
        
        let mut stats_map = HashMap::new();
        for (id, stats_result) in stats_results {
            match stats_result {
                Ok(Some(Ok(stats))) => {
                    stats_map.insert(id, stats);
                }
                Ok(Some(Err(e))) => {
                    eprintln!("Failed to get stats for container {}: {}", id, e);
                }
                Ok(None) => {
                    eprintln!("No stats available for container {}", id);
                }
                Err(_) => {
                    eprintln!("Timeout getting stats for container {}", id);
                }
            }
        }
        
        let mut container_infos = Vec::new();
        let mut current_container_stats = HashMap::new();
        
        for container in containers_list {
            let id_full = container.id.clone().unwrap_or_default();
            let id_short = id_full.get(..12).unwrap_or("N/A").to_string();
            
            let name = container.names
                .as_ref()
                .and_then(|names| names.first())
                .map(|name| name.strip_prefix('/').unwrap_or(name).to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            
            let status = container.status
                .as_deref()
                .unwrap_or("unknown")
                .to_string();
            
            let image = container.image
                .as_deref()
                .unwrap_or("unknown")
                .to_string();
            
            let ports = self.format_ports(&container.ports);
            
            let (cpu, mem, net_down, net_up, disk_r, disk_w) = 
                if let Some(stats) = stats_map.get(&id_full) {
                    self.calculate_container_metrics(
                        &id_full, 
                        stats, 
                        elapsed_secs,
                        &mut current_container_stats
                    )
                } else {
                    (
                        "0.00%".to_string(),
                        "0 B".to_string(),
                        "0 B/s".to_string(),
                        "0 B/s".to_string(),
                        "0 B/s".to_string(),
                        "0 B/s".to_string(),
                    )
                };
            
            container_infos.push(ContainerInfo {
                id: id_short,
                name,
                status,
                cpu,
                mem,
                net_down,
                net_up,
                disk_r,
                disk_w,
                image,
                ports,
            });
        }
        
        self.prev_container_stats = current_container_stats;
        Ok(container_infos)
    }
    
    #[cfg(feature = "docker")]
    fn calculate_container_metrics(
        &self,
        container_id: &str,
        stats: &bollard::container::Stats,
        elapsed_secs: f64,
        current_stats: &mut HashMap<String, ContainerIoStats>
    ) -> (String, String, String, String, String, String) {
        let prev_stats = self.prev_container_stats
            .get(container_id)
            .cloned()
            .unwrap_or_default();
        
        let mut container_io_stats = ContainerIoStats::default();
        
        let cpu_usage = self.calculate_cpu_usage(stats);
        let cpu_display = format!("{:.2}%", cpu_usage);
        
        let memory_usage = stats.memory_stats.usage.unwrap_or(0);
        let memory_display = format_size(memory_usage);
        
        if let Some(ref networks) = stats.networks {
            for (_, net_data) in networks {
                container_io_stats.net_rx += net_data.rx_bytes;
                container_io_stats.net_tx += net_data.tx_bytes;
            }
        }
        
        let net_rx_rate = calculate_rate(
            container_io_stats.net_rx,
            prev_stats.net_rx,
            elapsed_secs
        );
        let net_tx_rate = calculate_rate(
            container_io_stats.net_tx,
            prev_stats.net_tx,
            elapsed_secs
        );
        
        let net_down_display = format_rate(net_rx_rate);
        let net_up_display = format_rate(net_tx_rate);
        
        if let Some(ref blkio_stats) = stats.blkio_stats.io_service_bytes_recursive {
            for entry in blkio_stats {
                match entry.op.as_str() {
                    "Read" => container_io_stats.disk_r += entry.value,
                    "Write" => container_io_stats.disk_w += entry.value,
                    _ => {}
                }
            }
        }
        
        let disk_read_rate = calculate_rate(
            container_io_stats.disk_r,
            prev_stats.disk_r,
            elapsed_secs
        );
        let disk_write_rate = calculate_rate(
            container_io_stats.disk_w,
            prev_stats.disk_w,
            elapsed_secs
        );
        
        let disk_read_display = format_rate(disk_read_rate);
        let disk_write_display = format_rate(disk_write_rate);
        
        current_stats.insert(container_id.to_string(), container_io_stats);
        
        (
            cpu_display,
            memory_display,
            net_down_display,
            net_up_display,
            disk_read_display,
            disk_write_display,
        )
    }
    
    #[cfg(feature = "docker")]
    fn calculate_cpu_usage(&self, stats: &bollard::container::Stats) -> f64 {
        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage
            .saturating_sub(stats.precpu_stats.cpu_usage.total_usage) as f64;
        
        let system_delta = stats.cpu_stats.system_cpu_usage
            .unwrap_or(0)
            .saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0)) as f64;
        
        let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
        
        if system_delta > 0.0 && cpu_delta > 0.0 {
            (cpu_delta / system_delta) * num_cpus * 100.0
        } else {
            0.0
        }
    }
    
    #[cfg(feature = "docker")]
    fn format_ports(&self, ports: &Option<Vec<bollard::models::Port>>) -> String {
        if let Some(ports) = ports {
            let port_strings: Vec<String> = ports
                .iter()
                .filter_map(|port| {
                    if let Some(public_port) = port.public_port {
                        Some(format!("{}:{}", public_port, port.private_port))
                    } else {
                        Some(format!("{}", port.private_port))
                    }
                })
                .collect();
            
            if port_strings.is_empty() {
                "none".to_string()
            } else {
                port_strings.join(", ")
            }
        } else {
            "none".to_string()
        }
    }
    
    #[cfg(not(feature = "docker"))]
    async fn get_docker_containers(&mut self, _timeout_ms: u64) -> Result<Vec<ContainerInfo>, Box<dyn std::error::Error + Send + Sync>> {
        Err("Docker support not compiled".into())
    }
    
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "docker")]
        return self.docker.is_some();
        
        #[cfg(not(feature = "docker"))]
        false
    }
    
    pub async fn health_check(&self, timeout_ms: u64) -> bool {
        #[cfg(feature = "docker")]
        if let Some(ref docker) = self.docker {
            return timeout(
                Duration::from_millis(timeout_ms),
                docker.ping()
            ).await.is_ok();
        }
        
        false
    }
    
    pub async fn get_runtime_info(&self) -> Option<String> {
        #[cfg(feature = "docker")]
        if let Some(ref docker) = self.docker {
            if let Ok(version) = docker.version().await {
                return Some(format!(
                    "Docker {} (API {})",
                    version.version.unwrap_or_else(|| "unknown".to_string()),
                    version.api_version.unwrap_or_else(|| "unknown".to_string())
                ));
            }
        }
        
        None
    }
}

impl Default for ContainerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_container_monitor_creation() {
        let monitor = ContainerMonitor::new();
        // Just test that it doesn't panic
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_container_health_check() {
        let monitor = ContainerMonitor::new();
        // This will likely fail in test environment, but shouldn't panic
        let _result = monitor.health_check(1000).await;
        assert!(true);
    }
}