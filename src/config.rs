#![allow(dead_code)]

use clap::Parser;
use crate::types::AppConfig;
use crate::language::Language;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "puls")]
#[command(about = "A unified system monitoring and management tool for Linux")]
pub struct Cli {
    #[arg(short, long, default_value_t = false)]
    pub safe: bool,
    
    #[arg(short, long, default_value_t = 1000)]
    pub refresh: u64,
    
    #[arg(long, default_value_t = 60)]
    pub history: usize,
    
    #[arg(long, default_value_t = false)]
    pub show_system: bool,
    
    #[arg(long, default_value_t = false)]
    pub no_docker: bool,
    
    #[arg(long, default_value_t = false)]
    pub no_gpu: bool,
    
    #[arg(long, default_value_t = false)]
    pub no_network: bool,
    
    #[arg(long, default_value_t = false)]
    pub auto_scroll: bool,
    
    #[arg(long, default_value = "en")]
    pub lang: String,
    
    #[arg(long, default_value_t = false)]
    pub tr: bool,
    
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

impl From<Cli> for AppConfig {
    fn from(cli: Cli) -> Self {
        let language = if cli.tr {
            Language::Turkish
        } else {
            Language::from_str(&cli.lang)
        };
        
        Self {
            safe_mode: cli.safe,
            refresh_rate_ms: cli.refresh.max(100).min(10000), 
            history_length: cli.history.max(10).min(300),     
            enable_docker: !cli.safe && !cli.no_docker,
            enable_gpu_monitoring: !cli.safe && !cli.no_gpu,
            enable_network_monitoring: !cli.safe && !cli.no_network,
            language,
        }
    }
}

impl AppConfig {
    pub fn ui_refresh_rate_ms(&self) -> u64 {
        33  //30FPS i think
    }
    
    pub fn data_refresh_rate_ms(&self) -> u64 {
        self.refresh_rate_ms
    }
    
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "docker" => self.enable_docker,
            "gpu" => self.enable_gpu_monitoring,
            "network" => self.enable_network_monitoring,
            _ => true,
        }
    }
    
    pub fn get_collection_sleep_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.data_refresh_rate_ms())
    }
    
    pub fn get_operation_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.refresh_rate_ms / 2)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            safe_mode: false,
            refresh_rate_ms: 1000,
            history_length: 60,
            enable_docker: true,
            enable_gpu_monitoring: true,
            enable_network_monitoring: true,
            language: Language::English,
        }
    }
}

pub struct Features;

impl Features {
    #[cfg(feature = "docker")]
    pub const DOCKER: bool = true;
    #[cfg(not(feature = "docker"))]
    pub const DOCKER: bool = false;
    
    pub const NVIDIA_GPU: bool = true;
    pub const AMD_GPU: bool = true;
    
    pub fn has_gpu_support() -> bool {
        true
    }
    
    pub fn has_container_support() -> bool {
        Self::DOCKER
    }
}

pub struct PerformanceProfile {
    pub update_interval_ms: u64,
    pub history_size: usize,
    pub enable_expensive_ops: bool,
}

impl PerformanceProfile {
    pub fn detect() -> Self {
        let sys = sysinfo::System::new_all();
        let total_memory_gb = sys.total_memory() / (1024 * 1024 * 1024);
        
        if total_memory_gb >= 16 {
            Self {
                update_interval_ms: 500,
                history_size: 120,
                enable_expensive_ops: true,
            }
        } else if total_memory_gb >= 8 {
            Self {
                update_interval_ms: 1000,
                history_size: 60,
                enable_expensive_ops: true,
            }
        } else {
            Self {
                update_interval_ms: 2000,
                history_size: 30,
                enable_expensive_ops: false,
            }
        }
    }
    
    pub fn safe_mode() -> Self {
        Self {
            update_interval_ms: 2000,
            history_size: 30,
            enable_expensive_ops: false,
        }
    }
}