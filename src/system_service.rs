use std::process::Command;
use std::path::Path;
use std::io::Write;
use std::collections::{HashMap, HashSet};
use crate::types::{ServiceInfo, LogEntry, ConfigItem};

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
        crate::utils::get_current_uid() == 0
    }

    pub fn get_services(&self) -> Vec<ServiceInfo> {
        let mut services = Vec::new();
        let mut loaded_states = HashMap::new();
        let mut visited_services = HashSet::new();

        if let Ok(output) = Command::new("systemctl")
            .args(&["list-units", "--type=service", "--all", "--no-pager", "--no-legend", "--full"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0];
                    let active = parts[2];
                    
                    let description = if parts.len() > 4 {
                        parts[4..].join(" ")
                    } else {
                        format!("{} Service", name.replace(".service", ""))
                    };
                    
                    loaded_states.insert(name.to_string(), (active.to_string(), description));
                }
            }
        }

        if let Ok(output) = Command::new("systemctl")
            .args(&["list-unit-files", "--type=service", "--no-pager", "--no-legend", "--full"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    if !name.ends_with(".service") {
                        continue;
                    }
                    
                    visited_services.insert(name.to_string());
                    let state = parts[1];
                    let is_enabled = state == "enabled";
                    let (status_str, description) = if let Some((active, desc)) = loaded_states.get(name) {
                        let s = match active.as_str() {
                            "active" => "Running",
                            "activating" => "Starting",
                            "deactivating" => "Stopping",
                            "failed" => "Failed",
                            "reloading" => "Reloading",
                            _ => "Stopped",
                        };
                        (s.to_string(), desc.clone())
                    } else {
                        ("Stopped".to_string(), format!("{} Service", name.replace(".service", "")))
                    };

                    let is_running = status_str == "Running" || status_str == "Starting" || status_str == "Reloading";

                    services.push(ServiceInfo {
                        name: name.replace(".service", ""),
                        description,
                        status: status_str,
                        enabled: is_enabled,
                        can_start: !is_running && self.has_sudo,
                        can_stop: is_running && self.has_sudo,
                    });
                }
            }
        }
        
        for (name, (active, description)) in &loaded_states {
            if !visited_services.contains(name) {
                 if !name.ends_with(".service") {
                     continue;
                 }
                 
                 let status_str = if active == "active" { "Running" } else { "Stopped" };
                 let is_running = status_str == "Running";
                 
                 services.push(ServiceInfo {
                     name: name.replace(".service", ""),
                     description: description.clone(),
                     status: status_str.to_string(),
                     enabled: false,
                     can_start: !is_running && self.has_sudo,
                     can_stop: is_running && self.has_sudo,
                 });
            }
        }
        
        services.sort_by(|a, b| a.name.cmp(&b.name));

        services
    }

    pub fn start_service(&self, service_name: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = Command::new("systemctl")
            .args(&["start", &format!("{}.service", service_name)])
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
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = Command::new("systemctl")
            .args(&["stop", &format!("{}.service", service_name)])
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
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = Command::new("systemctl")
            .args(&["restart", &format!("{}.service", service_name)])
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
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = Command::new("systemctl")
            .args(&["enable", &format!("{}.service", service_name)])
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
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = Command::new("systemctl")
            .args(&["disable", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn get_service_status(&self, service_name: &str) -> String {
        let output = Command::new("systemctl")
            .args(&["status", &format!("{}.service", service_name), "--no-pager"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    String::from_utf8_lossy(&out.stderr).to_string()
                } else {
                    stdout.to_string()
                }
            }
            Err(e) => format!("Error getting status: {}", e),
        }
    }

    pub fn get_service_logs(&self, service_name: &str) -> String {
        let output = Command::new("journalctl")
            .args(&["-u", &format!("{}.service", service_name), "-n", "50", "--no-pager"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if stderr.trim().is_empty() {
                        "No logs found for this service.".to_string()
                    } else {
                        stderr.to_string()
                    }
                } else {
                    stdout.to_string()
                }
            }
            Err(e) => format!("Error getting service logs: {}", e),
        }
    }

    pub fn get_boots(&self) -> Vec<crate::types::BootInfo> {
        let mut boots = Vec::new();
        
        let output = match Command::new("journalctl")
            .arg("--list-boots")
            .output()
        {
            Ok(output) => output,
            Err(_) => return boots,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                 let boot_id = parts[1];
                 let date = parts[2..].join(" ");
                 
                 boots.push(crate::types::BootInfo {
                     id: boot_id.to_string(),
                     timestamp: date,
                 });
            }
        }
        
        boots
    }

    pub fn get_logs(&self, limit: usize, filter: Option<&str>, boot_id: Option<&str>) -> Vec<LogEntry> {
        let mut logs = Vec::new();

        let mut args = vec![
            "--no-pager".to_string(),
            "--output=short".to_string(),
        ];

        if limit > 0 {
            args.push("--lines".to_string());
            args.push(limit.to_string());
        }

        if let Some(f) = filter {
            if !f.is_empty() {
                args.push(format!("--grep={}", f));
            }
        }
        
        if let Some(bid) = boot_id {
            args.push(format!("--boot={}", bid));
        }

        let output = match Command::new("journalctl")
            .args(&args)
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
                            value: value.clone(),
                            original_value: value,
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
                original_value: hostname.trim().to_string(),
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
                    value: tz.clone(),
                    original_value: tz,
                    description: "System timezone".to_string(),
                    category: "System".to_string(),
                });
            }
        }

        configs
    }

    pub fn apply_config_changes(&self, changes: &[(String, String)]) -> Result<String, String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let mut output_log = String::new();
        let mut grub_changes = Vec::new();

        for (key, value) in changes {
            if key == "hostname" {
                self.set_hostname(value)
                    .map_err(|e| format!("Failed to set hostname: {}", e))?;
                output_log.push_str("[+] Successfully updated system hostname.\n");
            } else if key == "timezone" {
                self.set_timezone(value)
                    .map_err(|e| format!("Failed to set timezone: {}", e))?;
                output_log.push_str("[+] Successfully updated system timezone.\n");
            } else if key.starts_with("GRUB_") {
                grub_changes.push((key.clone(), value.clone()));
            }
        }

        if !grub_changes.is_empty() {
            let grub_file = "/etc/default/grub";
            let timestamp = crate::utils::current_formatted_time("%Y%m%d_%H%M%S");
            let backup_file = format!("{}.bak.{}", grub_file, timestamp);

            Command::new("cp")
                .args(&[grub_file, &backup_file])
                .output()
                .map_err(|e| format!("Failed to create backup of {}: {}", grub_file, e))?;

            output_log.push_str(&format!("[+] Created GRUB backup at {}.\n", backup_file));

            let content = std::fs::read_to_string(grub_file)
                .map_err(|e| format!("Failed to read {}: {}", grub_file, e))?;

            let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

            for (key, value) in &grub_changes {
                let mut found = false;
                for line in &mut lines {
                    if line.starts_with(&format!("{}=", key)) {
                        *line = format!("{}=\"{}\"", key, value);
                        found = true;
                        break;
                    }
                }
                if !found {
                    lines.push(format!("{}=\"{}\"", key, value));
                }
            }

            let new_content = lines.join("\n") + "\n";

            let mut child = Command::new("tee")
                .arg(grub_file)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to spawn tee to write GRUB config: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(new_content.as_bytes())
                    .map_err(|e| format!("Failed to write to GRUB config: {}", e))?;
            }
            child.wait().map_err(|e| format!("Failed waiting for tee: {}", e))?;

            output_log.push_str("[+] Successfully wrote changes to /etc/default/grub.\n");

            output_log.push_str("Compiling bootloader config...\n");
            let update_output = if Command::new("update-grub").output().is_ok() {
                let output = Command::new("update-grub")
                    .output()
                    .map_err(|e| format!("Failed to run update-grub: {}", e))?;
                format!("update-grub succeeded:\n{}", String::from_utf8_lossy(&output.stdout))
            } else if Command::new("grub-mkconfig").output().is_ok() {
                let output = Command::new("grub-mkconfig")
                    .args(&["-o", "/boot/grub/grub.cfg"])
                    .output()
                    .map_err(|e| format!("Failed to run grub-mkconfig: {}", e))?;
                format!("grub-mkconfig succeeded:\n{}", String::from_utf8_lossy(&output.stdout))
            } else if Command::new("grub2-mkconfig").output().is_ok() {
                let output = Command::new("grub2-mkconfig")
                    .args(&["-o", "/boot/grub2/grub.cfg"])
                    .output()
                    .map_err(|e| format!("Failed to run grub2-mkconfig: {}", e))?;
                format!("grub2-mkconfig succeeded:\n{}", String::from_utf8_lossy(&output.stdout))
            } else {
                return Err("No grub config update tool found (update-grub, grub-mkconfig, grub2-mkconfig)".to_string());
            };

            output_log.push_str(&update_output);
        }

        Ok(output_log)
    }

    pub fn set_hostname(&self, new_hostname: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        Command::new("hostnamectl")
            .args(&["set-hostname", new_hostname])
            .output()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn set_timezone(&self, timezone: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        Command::new("timedatectl")
            .args(&["set-timezone", timezone])
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