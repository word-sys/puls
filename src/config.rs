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
                    println!("  --lang <LANG>        Language setting (\"auto\", \"en\", \"tr\", \"fr\", \"de\", \"es\", \"it\", \"ru\")");
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
    pub temp_unit_fahrenheit: Option<bool>,
    pub default_tab: Option<usize>,
    pub process_tree_view: Option<bool>,
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
                            settings.theme = Some(t % crate::ui::colors::THEME_COUNT);
                        }
                    }
                    "refresh_rate_ms" | "refresh" => {
                        if let Ok(r) = val.parse::<u64>() {
                            settings.refresh_rate_ms = Some(r.clamp(100, 10000));
                        }
                    }
                    "temp_unit" | "temp_unit_fahrenheit" | "fahrenheit" => {
                        settings.temp_unit_fahrenheit = Some(val == "true" || val == "1" || val == "f" || val == "fahrenheit");
                    }
                    "default_tab" | "start_tab" => {
                        if let Ok(t) = val.parse::<usize>() {
                            settings.default_tab = Some(t.min(12));
                        }
                    }
                    "tree_view" | "process_tree_view" => {
                        settings.process_tree_view = Some(val == "true" || val == "1");
                    }
                    _ => {}
                }
            }
        }
    }
    settings
}

pub fn save_user_settings(
    language: Language,
    theme: usize,
    refresh_rate_ms: u64,
    temp_unit_fahrenheit: bool,
    default_tab: usize,
    process_tree_view: bool,
) -> Result<(), std::io::Error> {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.ini");
    let lang_str = language.code();
    let content = format!(
        "[puls]\nlanguage = {}\ntheme = {}\nrefresh_rate_ms = {}\ntemp_unit_fahrenheit = {}\ndefault_tab = {}\nprocess_tree_view = {}\n",
        lang_str, theme, refresh_rate_ms, temp_unit_fahrenheit, default_tab, process_tree_view
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
            cli.refresh.clamp(100, 10000)
        } else {
            saved.refresh_rate_ms.unwrap_or(1000)
        };

        let theme = saved.theme.unwrap_or(0);
        let temp_unit_fahrenheit = saved.temp_unit_fahrenheit.unwrap_or(false);
        let default_tab = saved.default_tab.unwrap_or(0);
        let process_tree_view = saved.process_tree_view.unwrap_or(false);
        
        Self {
            safe_mode: cli.safe,
            refresh_rate_ms, 
            history_length: cli.history.clamp(10, 300),     
            enable_docker: !cli.safe && !cli.no_docker,
            enable_gpu_monitoring: !cli.safe && !cli.no_gpu,
            enable_network_monitoring: !cli.safe && !cli.no_network,
            language,
            theme,
            telemetry: cli.telemetry,
            temp_unit_fahrenheit,
            default_tab,
            process_tree_view,
        }
    }
}

impl AppConfig {
    pub fn get_operation_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.refresh_rate_ms / 2)
    }

    pub fn ui_refresh_rate_ms(&self) -> u64 {
        33
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
            temp_unit_fahrenheit: false,
            default_tab: 0,
            process_tree_view: false,
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
        std::fs::write(&config_file, "[puls]\nlanguage = tr\ntheme = 2\nrefresh_rate_ms = 500\ntemp_unit_fahrenheit = true\ndefault_tab = 2\nprocess_tree_view = true\n").unwrap();

        let parsed = parse_settings_from_path(&config_file);
        assert_eq!(parsed.language, Some(Language::Turkish));
        assert_eq!(parsed.theme, Some(2));
        assert_eq!(parsed.refresh_rate_ms, Some(500));
        assert_eq!(parsed.temp_unit_fahrenheit, Some(true));
        assert_eq!(parsed.default_tab, Some(2));
        assert_eq!(parsed.process_tree_view, Some(true));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
