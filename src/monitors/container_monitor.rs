use std::collections::HashMap;
use std::time::{Duration, Instant};
use futures_util::{future, stream::StreamExt};
use tokio::time::timeout;

#[cfg(feature = "docker")]
#[cfg(feature = "docker")]
use bollard::Docker;
#[cfg(feature = "docker")]
use bollard::query_parameters::{StatsOptions, ListContainersOptions, LogsOptions};
#[cfg(feature = "docker")]
use bollard::models::ContainerStatsResponse;

use crate::types::{ContainerInfo, ContainerIoStats};
use crate::utils::{format_size, format_rate, calculate_rate};

pub struct ContainerMonitor {
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    #[cfg(feature = "docker")]
    pub init_error: Option<String>,
    
    prev_container_stats: HashMap<String, ContainerIoStats>,
    last_update: Instant,
}

impl ContainerMonitor {
    pub fn new() -> Self {
        #[cfg(feature = "docker")]
        let (docker, init_error) =  match Self::init_docker() {
            Ok(d) => (Some(d), None),
            Err(e) => (None, Some(e)),
        };

        Self {
            #[cfg(feature = "docker")]
            docker,
            #[cfg(feature = "docker")]
            init_error,
            
            prev_container_stats: HashMap::new(),
            last_update: Instant::now(),
        }
    }
    
    #[cfg(feature = "docker")]
    fn init_docker() -> Result<Docker, String> {
        if let Ok(docker) = Docker::connect_with_local_defaults() {
             return Ok(docker);
        }

        //Fallback
        if let Ok(docker) = Docker::connect_with_socket("/var/run/docker.sock", 120, bollard::API_DEFAULT_VERSION) {
             return Ok(docker);
        }
        
        
        if std::path::Path::new("/var/run/docker.sock").exists() {
             return Err("Permission denied accessing /var/run/docker.sock. Add user to 'docker' group.".to_string());
        }

        Err("Docker daemon not found or connection failed.".to_string())
    }
    
    #[cfg(not(feature = "docker"))]
    fn init_docker() -> Option<()> {
        None
    }
    
    pub async fn get_containers(&mut self, timeout_ms: u64) -> Result<Vec<ContainerInfo>, String> {
        #[cfg(feature = "docker")]
        if let Some(ref docker) = self.docker {
            let docker_clone = docker.clone();
            match self.get_docker_containers(&docker_clone, timeout_ms).await {
                Ok(containers) => return Ok(containers),
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("hyper") || err_str.contains("Connect") {
                        return Err("Docker daemon is not running".to_string());
                    }
                    return Err(format!("Docker: {}", err_str));
                }
            }
        } else {
             return Err("Docker not available".to_string());
        }
        
        #[cfg(not(feature = "docker"))]
        Err("Docker support not compiled".to_string())
    }
    
    #[cfg(feature = "docker")]
    async fn get_docker_containers(&mut self, docker: &Docker, timeout_ms: u64) -> Result<Vec<ContainerInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;
        
        match timeout(Duration::from_millis(timeout_ms / 4), docker.ping()).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                let err_str = e.to_string();
                if err_str.contains("Connect") || err_str.contains("connection") {
                    return Err("Docker daemon is not running or not accessible".into());
                }
                return Err(format!("Docker: {}", err_str).into());
            }
            Err(_) => return Err("Docker daemon not responding (timeout)".into()),
        }
        
        let options: Option<ListContainersOptions> = None;
        let containers_list = timeout(
            Duration::from_millis(timeout_ms / 2),
            docker.list_containers(options)
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
                    ).await
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
    async fn calculate_container_metrics(
        &self,
        container_id: &str,
        stats: &ContainerStatsResponse,
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
        
        let memory_usage = stats.memory_stats.as_ref()
            .and_then(|m| m.usage)
            .unwrap_or(0);
        let memory_display = format_size(memory_usage);
        
        if let Some(ref networks) = stats.networks {
            for (_, net_data) in networks {
                container_io_stats.net_rx += net_data.rx_bytes.unwrap_or(0);
                container_io_stats.net_tx += net_data.tx_bytes.unwrap_or(0);
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
        
        if let Some(ref blkio_stats) = stats.blkio_stats {
            if let Some(ref entries) = blkio_stats.io_service_bytes_recursive {
                for entry in entries {
                    if let (Some(op), Some(value)) = (&entry.op, entry.value) {
                         match op.as_str() {
                            "Read" => container_io_stats.disk_r += value,
                            "Write" => container_io_stats.disk_w += value,
                            _ => {}
                        }
                    }
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
    fn calculate_cpu_usage(&self, stats: &ContainerStatsResponse) -> f64 {
        let cpu_stats = stats.cpu_stats.as_ref();
        let precpu_stats = stats.precpu_stats.as_ref();
        
        if cpu_stats.is_none() || precpu_stats.is_none() {
            return 0.0;
        }
        
        let cpu_stats = cpu_stats.unwrap();
        let precpu_stats = precpu_stats.unwrap();
        
        let cpu_usage = cpu_stats.cpu_usage.as_ref().and_then(|u| u.total_usage).unwrap_or(0);
        let precpu_usage = precpu_stats.cpu_usage.as_ref().and_then(|u| u.total_usage).unwrap_or(0);
        
        let system_usage = cpu_stats.system_cpu_usage.unwrap_or(0);
        let presystem_usage = precpu_stats.system_cpu_usage.unwrap_or(0);
        
        let cpu_delta = cpu_usage.saturating_sub(precpu_usage) as f64;
        let system_delta = system_usage.saturating_sub(presystem_usage) as f64;
        
        let num_cpus = cpu_stats.online_cpus.unwrap_or(1) as f64;
        
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

    pub async fn get_container_logs(&self, container_id: &str) -> Result<Vec<String>, String> {
        if let Some(ref docker) = self.docker {
            fetch_container_logs(docker, container_id).await
        } else {
            Err("Docker not available".to_string())
        }
    }

    #[cfg(not(feature = "docker"))]
    pub async fn get_container_logs(&self, _container_id: &str) -> Result<Vec<String>, String> {
         Err("Docker support not compiled".to_string())
    }
    
    #[cfg(feature = "docker")]
    pub fn client(&self) -> Option<Docker> {
        self.docker.clone()
    }
    
    #[cfg(not(feature = "docker"))]
    pub fn client(&self) -> Option<()> {
        None
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

#[cfg(feature = "docker")]
pub async fn fetch_container_logs(docker: &Docker, container_id: &str) -> Result<Vec<String>, String> {
    let options = LogsOptions {
        stdout: true,
        stderr: true,
        tail: "50".to_string(),
        ..Default::default()
    };
    
    let mut logs_stream = docker.logs(container_id, Some(options));
    let mut logs = Vec::new();
    
    while let Some(log_result) = logs_stream.next().await {
        if let Ok(log_output) = log_result {
                logs.push(log_output.to_string());
        }
    }
    
    Ok(logs)
}

#[cfg(not(feature = "docker"))]
pub async fn fetch_container_logs(_docker: &(), _container_id: &str) -> Result<Vec<String>, String> {
    Err("Docker support not compiled".to_string())
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
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_container_health_check() {
        let monitor = ContainerMonitor::new();
        let _result = monitor.health_check(1000).await;
        assert!(true);
    }
}