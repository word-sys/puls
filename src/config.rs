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

pub fn get_config_dir() -> std::path::PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return std::path::PathBuf::from(xdg).join("puls");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".config").join("puls");
    }
    std::path::PathBuf::from(".config").join("puls")
}

pub fn get_config_file_path() -> std::path::PathBuf {
    get_config_dir().join("config.ini")
}

#[derive(Debug, Clone, Default)]
pub struct UserSettings {
    pub language: Option<Language>,
    pub theme: Option<usize>,
    pub refresh_rate_ms: Option<u64>,
}

pub fn load_user_settings() -> UserSettings {
    let path = get_config_file_path();
    parse_settings_from_path(&path)
}

pub fn parse_settings_from_path(path: &std::path::Path) -> UserSettings {
    let mut settings = UserSettings::default();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let val = val.trim();
                match key.as_str() {
                    "language" | "lang" => {
                        settings.language = Some(Language::from_str(val));
                    }
                    "theme" => {
                        if let Ok(t) = val.parse::<usize>() {
                            settings.theme = Some(t % 3);
                        }
                    }
                    "refresh_rate_ms" | "refresh" => {
                        if let Ok(r) = val.parse::<u64>() {
                            settings.refresh_rate_ms = Some(r.max(100).min(10000));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    settings
}

pub fn save_user_settings(language: Language, theme: usize, refresh_rate_ms: u64) -> Result<(), std::io::Error> {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.ini");
    let lang_str = match language {
        Language::English => "en",
        Language::Turkish => "tr",
    };
    let content = format!(
        "[puls]\nlanguage = {}\ntheme = {}\nrefresh_rate_ms = {}\n",
        lang_str, theme, refresh_rate_ms
    );
    std::fs::write(&path, content)?;
    Ok(())
}

impl From<Cli> for AppConfig {
    fn from(cli: Cli) -> Self {
        let saved = load_user_settings();

        let language = if cli.tr {
            Language::Turkish
        } else if cli.lang != "auto" {
            Language::from_str(&cli.lang)
        } else if let Some(saved_lang) = saved.language {
            saved_lang
        } else {
            Language::detect()
        };

        let refresh_rate_ms = if cli.refresh != 1000 {
            cli.refresh.max(100).min(10000)
        } else if let Some(saved_refresh) = saved.refresh_rate_ms {
            saved_refresh
        } else {
            1000
        };

        let theme = saved.theme.unwrap_or(0);
        
        Self {
            safe_mode: cli.safe,
            refresh_rate_ms, 
            history_length: cli.history.max(10).min(300),     
            enable_docker: !cli.safe && !cli.no_docker,
            enable_gpu_monitoring: !cli.safe && !cli.no_gpu,
            enable_network_monitoring: !cli.safe && !cli.no_network,
            language,
            theme,
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
            theme: 0,
            telemetry: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_settings() {
        let temp_dir = std::env::temp_dir().join(format!("puls_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let config_file = temp_dir.join("config.ini");
        std::fs::write(&config_file, "[puls]\nlanguage = tr\ntheme = 2\nrefresh_rate_ms = 500\n").unwrap();

        let parsed = parse_settings_from_path(&config_file);
        assert_eq!(parsed.language, Some(Language::Turkish));
        assert_eq!(parsed.theme, Some(2));
        assert_eq!(parsed.refresh_rate_ms, Some(500));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
