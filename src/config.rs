use clap::Parser;
use sysinfo::{System, SystemExt};
use crate::types::AppConfig;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "puls")]
#[command(about = "A comprehensive system monitoring tool")]
pub struct Cli {
    /// Enable safe mode (disable Docker and GPU monitoring)
    #[arg(short, long, default_value_t = false)]
    pub safe: bool,
    
    /// Refresh rate in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    pub refresh: u64,
    
    /// History length for graphs
    #[arg(long, default_value_t = 60)]
    pub history: usize,
    
    /// Show system processes
    #[arg(long, default_value_t = false)]
    pub show_system: bool,
    
    /// Disable Docker monitoring
    #[arg(long, default_value_t = false)]
    pub no_docker: bool,
    
    /// Disable GPU monitoring
    #[arg(long, default_value_t = false)]
    pub no_gpu: bool,
    
    /// Disable network monitoring
    #[arg(long, default_value_t = false)]
    pub no_network: bool,
    
    /// Enable auto-scroll in process list
    #[arg(long, default_value_t = false)]
    pub auto_scroll: bool,
    
    /// Enable verbose logging
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

impl From<Cli> for AppConfig {
    fn from(cli: Cli) -> Self {
        Self {
            safe_mode: cli.safe,
            refresh_rate_ms: cli.refresh.max(100).min(10000), // Clamp between 100ms and 10s
            history_length: cli.history.max(10).min(300),     // Clamp between 10 and 300 entries
            enable_docker: !cli.safe && !cli.no_docker,
            enable_gpu_monitoring: !cli.safe && !cli.no_gpu,
            enable_network_monitoring: !cli.safe && !cli.no_network,
            show_system_processes: cli.show_system,
            auto_scroll: cli.auto_scroll,
        }
    }
}

impl AppConfig {
    /// Get the UI refresh rate (should be faster than data refresh for smooth UI)
    pub fn ui_refresh_rate_ms(&self) -> u64 {
        // UI refreshes at 60 FPS (16.67ms) for smooth experience
        16
    }
    
    /// Get data collection rate (slower than UI for efficiency)
    pub fn data_refresh_rate_ms(&self) -> u64 {
        self.refresh_rate_ms
    }
    
    /// Check if feature is enabled based on safe mode and specific flags
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "docker" => self.enable_docker,
            "gpu" => self.enable_gpu_monitoring,
            "network" => self.enable_network_monitoring,
            _ => true,
        }
    }
    
    /// Get appropriate thread sleep duration for data collection
    pub fn get_collection_sleep_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.data_refresh_rate_ms())
    }
    
    /// Get appropriate timeout for async operations
    pub fn get_operation_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.refresh_rate_ms / 2)
    }
}

/// Default configuration for production use
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            safe_mode: false,
            refresh_rate_ms: 1000,
            history_length: 60,
            enable_docker: true,
            enable_gpu_monitoring: true,
            enable_network_monitoring: true,
            show_system_processes: false,
            auto_scroll: false,
        }
    }
}

/// Feature flags for conditional compilation
pub struct Features;

impl Features {
    #[cfg(feature = "docker")]
    pub const DOCKER: bool = true;
    #[cfg(not(feature = "docker"))]
    pub const DOCKER: bool = false;
    
    #[cfg(feature = "nvidia-gpu")]
    pub const NVIDIA_GPU: bool = true;
    #[cfg(not(feature = "nvidia-gpu"))]
    pub const NVIDIA_GPU: bool = false;
    
    #[cfg(feature = "amd-gpu")]
    pub const AMD_GPU: bool = true;
    #[cfg(not(feature = "amd-gpu"))]
    pub const AMD_GPU: bool = false;
    
    /// Check if any GPU monitoring is available
    pub fn has_gpu_support() -> bool {
        Self::NVIDIA_GPU || Self::AMD_GPU
    }
    
    /// Check if container monitoring is available
    pub fn has_container_support() -> bool {
        Self::DOCKER
    }
}

/// Performance settings based on system capabilities
pub struct PerformanceProfile {
    pub update_interval_ms: u64,
    pub history_size: usize,
    pub enable_expensive_ops: bool,
}

impl PerformanceProfile {
    /// Detect appropriate performance profile based on system
    pub fn detect() -> Self {
        // Simple heuristic based on available memory
        let sys = sysinfo::System::new_all();
        let total_memory_gb = sys.total_memory() / (1024 * 1024 * 1024);
        
        if total_memory_gb >= 16 {
            // High-end system
            Self {
                update_interval_ms: 500,
                history_size: 120,
                enable_expensive_ops: true,
            }
        } else if total_memory_gb >= 8 {
            // Mid-range system
            Self {
                update_interval_ms: 1000,
                history_size: 60,
                enable_expensive_ops: true,
            }
        } else {
            // Low-end system or constrained environment
            Self {
                update_interval_ms: 2000,
                history_size: 30,
                enable_expensive_ops: false,
            }
        }
    }
    
    /// Override settings for safe mode
    pub fn safe_mode() -> Self {
        Self {
            update_interval_ms: 2000,
            history_size: 30,
            enable_expensive_ops: false,
        }
    }
}