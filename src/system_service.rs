use std::process::Command;
use std::path::Path;
use std::io::Write;
use crate::types::{ServiceInfo, LogEntry, ConfigItem};
use chrono::Local;

pub struct SystemManager {
    has_sudo: bool,
}

impl SystemManager {
    pub fn new() -> Self {
        let has_sudo = Self::check_sudo();
        SystemManager { has_sudo }
    }

    pub fn has_sudo_privileges(&self) -> bool {
        self.has_sudo
    }

    fn check_sudo() -> bool {
        match Command::new("sudo")
            .arg("-n")
            .arg("true")
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub fn get_services(&self) -> Vec<ServiceInfo> {
        let mut services = Vec::new();

        let output = match Command::new("systemctl")
            .args(&["list-units", "--type=service", "--all", "--no-pager", "--quiet"])
            .output()
        {
            Ok(output) => output,
            Err(_) => return services,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines().take(20) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let service_name = parts[0];
                if !service_name.ends_with(".service") {
                    continue;
                }

                let status = parts[3];
                let active = parts[2];

                let enabled_output = Command::new("systemctl")
                    .args(&["is-enabled", service_name])
                    .output();

                let is_enabled = match enabled_output {
                    Ok(output) => String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .to_string() == "enabled",
                    Err(_) => false,
                };

                let is_running = active == "active";

                services.push(ServiceInfo {
                    name: service_name.replace(".service", ""),
                    description: format!("{} Service", service_name.replace(".service", "")),
                    status: if is_running {
                        "Running".to_string()
                    } else {
                        "Stopped".to_string()
                    },
                    enabled: is_enabled,
                    can_start: !is_running && self.has_sudo,
                    can_stop: is_running && self.has_sudo,
                });
            }
        }

        services
    }

    pub fn start_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let output = Command::new("sudo")
            .args(&["systemctl", "start", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn stop_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let output = Command::new("sudo")
            .args(&["systemctl", "stop", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn restart_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let output = Command::new("sudo")
            .args(&["systemctl", "restart", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn enable_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let output = Command::new("sudo")
            .args(&["systemctl", "enable", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn disable_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let output = Command::new("sudo")
            .args(&["systemctl", "disable", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn get_logs(&self, limit: usize) -> Vec<LogEntry> {
        let mut logs = Vec::new();

        let output = match Command::new("journalctl")
            .args(&[
                "--lines",
                &limit.to_string(),
                "--no-pager",
                "--output=short",
            ])
            .output()
        {
            Ok(output) => output,
            Err(_) => return logs,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, ' ').collect();

            if parts.len() >= 3 {
                let timestamp = format!("{} {}", parts.get(0).unwrap_or(&""), parts.get(1).unwrap_or(&""));
                let service_and_msg = parts.get(3).unwrap_or(&"");
                let (service, message) = if let Some(colon_pos) = service_and_msg.find(':') {
                    let svc = &service_and_msg[..colon_pos];
                    let msg = &service_and_msg[colon_pos + 1..].trim();
                    (svc.to_string(), msg.to_string())
                } else {
                    (service_and_msg.to_string(), String::new())
                };

                let level = if message.to_uppercase().contains("ERROR") {
                    "ERROR"
                } else if message.to_uppercase().contains("WARN") {
                    "WARNING"
                } else if message.to_uppercase().contains("FAIL") || message.to_uppercase().contains("FAILED") {
                    "ERROR"
                } else {
                    "INFO"
                };

                logs.push(LogEntry {
                    timestamp,
                    level: level.to_string(),
                    service: service.replace("[pid]", ""),
                    message,
                });
            }
        }

        logs
    }

    pub fn get_grub_config(&self) -> Vec<ConfigItem> {
        let mut configs = Vec::new();
        let grub_file = "/etc/default/grub";

        if !Path::new(grub_file).exists() {
            return configs;
        }

        if let Ok(content) = std::fs::read_to_string(grub_file) {
            for line in content.lines() {
                if line.starts_with("GRUB_") && !line.starts_with('#') {
                    if let Some(pos) = line.find('=') {
                        let key = line[..pos].to_string();
                        let mut value = line[pos + 1..].to_string();

                        if value.starts_with('"') && value.ends_with('"') {
                            value = value[1..value.len() - 1].to_string();
                        }

                        configs.push(ConfigItem {
                            key,
                            value,
                            description: "GRUB boot parameter".to_string(),
                            category: "GRUB".to_string(),
                        });
                    }
                }
            }
        }

        let hostname_file = "/etc/hostname";
        if let Ok(hostname) = std::fs::read_to_string(hostname_file) {
            configs.push(ConfigItem {
                key: "hostname".to_string(),
                value: hostname.trim().to_string(),
                description: "System hostname".to_string(),
                category: "System".to_string(),
            });
        }

        if let Ok(tz_output) = Command::new("timedatectl")
            .arg("show")
            .arg("--value")
            .arg("--property=Timezone")
            .output()
        {
            let tz = String::from_utf8_lossy(&tz_output.stdout).trim().to_string();
            if !tz.is_empty() {
                configs.push(ConfigItem {
                    key: "timezone".to_string(),
                    value: tz,
                    description: "System timezone".to_string(),
                    category: "System".to_string(),
                });
            }
        }

        configs
    }

    pub fn set_grub_config(&self, key: &str, value: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        let grub_file = "/etc/default/grub";

        let content = std::fs::read_to_string(grub_file)
            .map_err(|e| e.to_string())?;

        let mut new_content = String::new();
        let mut found = false;

        for line in content.lines() {
            if line.starts_with(&format!("{}=", key)) {
                new_content.push_str(&format!("{}=\"{}\"\n", key, value));
                found = true;
            } else {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        if !found {
            new_content.push_str(&format!("{}=\"{}\"\n", key, value));
        }

        let mut child = Command::new("sudo")
            .arg("tee")
            .arg(grub_file)
            .arg("/dev/null")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(new_content.as_bytes())
                .map_err(|e| e.to_string())?;
        }

        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_hostname(&self, new_hostname: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        Command::new("sudo")
            .args(&["hostnamectl", "set-hostname", new_hostname])
            .output()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn set_timezone(&self, timezone: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (sudo required)".to_string());
        }

        Command::new("sudo")
            .args(&["timedatectl", "set-timezone", timezone])
            .output()
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

impl Default for SystemManager {
    fn default() -> Self {
        Self::new()
    }
}