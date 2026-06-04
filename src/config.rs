#![allow(dead_code)]

use crate::types::AppConfig;
use crate::language::Language;

#[derive(Debug, Clone)]
pub struct Cli {
    pub safe: bool,
    pub refresh: u64,
    pub history: usize,
    pub show_system: bool,
    pub no_docker: bool,
    pub no_gpu: bool,
    pub no_network: bool,
    pub auto_scroll: bool,
    pub lang: String,
    pub tr: bool,
    pub verbose: bool,
    pub telemetry: bool,
}

impl Cli {
    pub fn parse() -> Self {
        let mut cli = Self {
            safe: false,
            refresh: 1000,
            history: 60,
            show_system: false,
            no_docker: false,
            no_gpu: false,
            no_network: false,
            auto_scroll: false,
            lang: "auto".to_string(),
            tr: false,
            verbose: false,
            telemetry: false,
        };

        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];
            match arg.as_str() {
                "-h" | "--help" => {
                    println!("puls v{} - A unified system monitoring and management tool for Linux", env!("CARGO_PKG_VERSION"));
                    println!("\nUsage: puls [OPTIONS]");
                    println!("\nOptions:");
                    println!("  -s, --safe           Enable safe mode (read-only diagnostics, disable destructive system commands)");
                    println!("  -r, --refresh <MS>   Refresh rate in milliseconds (default: 1000)");
                    println!("  --history <SIZE>     History log/chart length (default: 60)");
                    println!("  --show-system        Show system processes (default: false)");
                    println!("  --no-docker          Disable Docker containers monitoring");
                    println!("  --no-gpu             Disable GPU usage queries");
                    println!("  --no-network         Disable advanced network metrics collection");
                    println!("  --auto-scroll        Enable automatic scroll for logs");
                    println!("  --lang <LANG>        Language setting (\"auto\", \"en\", \"tr\")");
                    println!("  --tr                 Shortcut for Turkish language");
                    println!("  -v, --verbose        Enable verbose stderr error logging");
                    println!("  --telemetry          Show initialization telemetry");
                    println!("  -h, --help           Print help information");
                    std::process::exit(0);
                }
                "-s" | "--safe" => {
                    cli.safe = true;
                }
                "-r" | "--refresh" => {
                    if i + 1 < args.len() {
                        i += 1;
                        if let Ok(r) = args[i].parse::<u64>() {
                            cli.refresh = r;
                        }
                    }
                }
                "--history" => {
                    if i + 1 < args.len() {
                        i += 1;
                        if let Ok(h) = args[i].parse::<usize>() {
                            cli.history = h;
                        }
                    }
                }
                "--show-system" => {
                    cli.show_system = true;
                }
                "--no-docker" => {
                    cli.no_docker = true;
                }
                "--no-gpu" => {
                    cli.no_gpu = true;
                }
                "--no-network" => {
                    cli.no_network = true;
                }
                "--auto-scroll" => {
                    cli.auto_scroll = true;
                }
                "--lang" => {
                    if i + 1 < args.len() {
                        i += 1;
                        cli.lang = args[i].clone();
                    }
                }
                "--tr" => {
                    cli.tr = true;
                }
                "-v" | "--verbose" => {
                    cli.verbose = true;
                }
                "--telemetry" => {
                    cli.telemetry = true;
                }
                other if other.starts_with('-') && !other.starts_with("--") => {
                    for c in other.chars().skip(1) {
                        match c {
                            's' => cli.safe = true,
                            'v' => cli.verbose = true,
                            'h' => {
                                println!("puls v{} - A unified system monitoring and management tool for Linux", env!("CARGO_PKG_VERSION"));
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        cli
    }
}

impl From<Cli> for AppConfig {
    fn from(cli: Cli) -> Self {
        let language = if cli.tr {
            Language::Turkish
        } else if cli.lang == "auto" {
            Language::detect()
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
            telemetry: cli.telemetry,
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
            telemetry: false,
        }
    }
}
