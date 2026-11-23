use std::process::Command;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use crate::sys::ffi;
use crate::sys::unix_http::JsonValue;
use crate::types::{BootBlameEntry, ServiceInfo, LogEntry, ConfigItem, SystemTimerInfo, UserSessionInfo};

pub struct SystemManager {
    has_sudo: bool,
    user_scope: bool,
}

impl SystemManager {
    pub fn new() -> Self {
        let has_sudo = Self::check_sudo();
        SystemManager { has_sudo, user_scope: false }
    }

    pub fn with_user_scope(user_scope: bool) -> Self {
        let has_sudo = Self::check_sudo();
        SystemManager { has_sudo, user_scope }
    }

    pub fn has_sudo_privileges(&self) -> bool {
        self.has_sudo
    }

    fn can_manage(&self) -> bool {
        self.user_scope || self.has_sudo
    }

    fn systemctl_cmd(&self) -> Command {
        let mut cmd = Command::new("systemctl");
        if self.user_scope {
            cmd.arg("--user");
        }
        cmd
    }

    fn check_sudo() -> bool {
        crate::utils::get_current_uid() == 0
    }

    pub fn get_services(&self) -> Vec<ServiceInfo> {
        let mut services = Vec::new();
        let mut loaded_states = HashMap::new();
        let mut visited_services = HashSet::new();

        if let Ok(output) = self
            .systemctl_cmd()
            .args(["list-units", "--type=service", "--all", "--no-pager", "--no-legend", "--full"])
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

        if let Ok(output) = self
            .systemctl_cmd()
            .args(["list-unit-files", "--type=service", "--no-pager", "--no-legend", "--full"])
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
                    let status_str = if let Some((active, _desc)) = loaded_states.get(name) {
                        let s = match active.as_str() {
                            "active" => "Running",
                            "activating" => "Starting",
                            "deactivating" => "Stopping",
                            "failed" => "Failed",
                            "reloading" => "Reloading",
                            _ => "Stopped",
                        };
                        s.to_string()
                    } else {
                        "Stopped".to_string()
                    };

                    let is_running = status_str == "Running" || status_str == "Starting" || status_str == "Reloading";
                    let manageable = self.can_manage();

                    services.push(ServiceInfo {
                        name: name.replace(".service", ""),
                        status: status_str,
                        enabled: is_enabled,
                        can_start: !is_running && manageable,
                        can_stop: is_running && manageable,
                    });
                }
            }
        }
        
        for (name, (active, _description)) in &loaded_states {
            if !visited_services.contains(name) {
                 if !name.ends_with(".service") {
                     continue;
                 }
                 
                 let status_str = if active == "active" { "Running" } else { "Stopped" };
                 let is_running = status_str == "Running";
                 let manageable = self.can_manage();
                 
                 services.push(ServiceInfo {
                     name: name.replace(".service", ""),
                     status: status_str.to_string(),
                     enabled: false,
                     can_start: !is_running && manageable,
                     can_stop: is_running && manageable,
                 });
            }
        }
        
        services.sort_by(|a, b| a.name.cmp(&b.name));

        services
    }

    pub fn start_service(&self, service_name: &str) -> Result<(), String> {
        if !self.can_manage() {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = self
            .systemctl_cmd()
            .args(["start", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn stop_service(&self, service_name: &str) -> Result<(), String> {
        if !self.can_manage() {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = self
            .systemctl_cmd()
            .args(["stop", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn restart_service(&self, service_name: &str) -> Result<(), String> {
        if !self.can_manage() {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = self
            .systemctl_cmd()
            .args(["restart", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn enable_service(&self, service_name: &str) -> Result<(), String> {
        if !self.can_manage() {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = self
            .systemctl_cmd()
            .args(["enable", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn disable_service(&self, service_name: &str) -> Result<(), String> {
        if !self.can_manage() {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let output = self
            .systemctl_cmd()
            .args(["disable", &format!("{}.service", service_name)])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn get_service_status(&self, service_name: &str) -> String {
        let output = self
            .systemctl_cmd()
            .args(["status", &format!("{}.service", service_name), "--no-pager"])
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

    pub fn get_service_unit_file(&self, service_name: &str) -> String {
        let unit_file = format!("{}.service", service_name);
        let mut search_dirs: Vec<PathBuf> = Vec::new();
        if self.user_scope {
            if let Ok(home) = std::env::var("HOME") {
                search_dirs.push(PathBuf::from(&home).join(".config/systemd/user"));
            }
            search_dirs.push(PathBuf::from("/etc/systemd/user"));
            search_dirs.push(PathBuf::from("/usr/lib/systemd/user"));
        } else {
            search_dirs.push(PathBuf::from("/etc/systemd/system"));
            search_dirs.push(PathBuf::from("/run/systemd/system"));
            search_dirs.push(PathBuf::from("/usr/lib/systemd/system"));
            search_dirs.push(PathBuf::from("/lib/systemd/system"));
        }

        for dir in search_dirs {
            let candidate = dir.join(&unit_file);
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                return format!("# {}\n\n{}", candidate.display(), content);
            }
        }

        match self.systemctl_cmd().args(["cat", &unit_file, "--no-pager"]).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
            Ok(out) => String::from_utf8_lossy(&out.stderr).into_owned(),
            Err(e) => format!("Unit file not found: {}", e),
        }
    }

    pub fn get_service_logs(&self, service_name: &str) -> String {
        let mut cmd = Command::new("journalctl");
        if self.user_scope {
            cmd.arg("--user");
        }
        let output = cmd
            .args(["-u", &format!("{}.service", service_name), "-n", "50", "--no-pager"])
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

    pub fn get_boot_blame(&self) -> Vec<BootBlameEntry> {
        let mut cmd = Command::new("systemd-analyze");
        if self.user_scope {
            cmd.arg("--user");
        }
        cmd.args(["blame", "--no-pager"]);
        let Ok(out) = cmd.output() else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        parse_boot_blame(&String::from_utf8_lossy(&out.stdout))
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
        let mut base_args = vec!["--no-pager".to_string()];

        if limit > 0 {
            base_args.push("--lines".to_string());
            base_args.push(limit.to_string());
        }

        if let Some(f) = filter {
            if !f.is_empty() {
                base_args.push(format!("--grep={}", f));
            }
        }
        
        if let Some(bid) = boot_id {
            base_args.push(format!("--boot={}", bid));
        } else {
            base_args.push("--boot=0".to_string());
        }

        let mut json_args = base_args.clone();
        json_args.push("--output=json".to_string());
        if let Ok(output) = Command::new("journalctl").args(&json_args).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parsed: Vec<LogEntry> = stdout
                    .lines()
                    .filter_map(parse_journal_json_line)
                    .collect();
                if !parsed.is_empty() {
                    return parsed;
                }
            }
        }

        let mut short_args = base_args;
        short_args.push("--output=short".to_string());
        let mut logs = Vec::new();
        let output = match Command::new("journalctl").args(&short_args).output() {
            Ok(output) => output,
            Err(_) => return logs,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() >= 3 {
                let timestamp = format!("{} {}", parts.first().unwrap_or(&""), parts.get(1).unwrap_or(&""));
                let service_and_msg = parts.get(3).unwrap_or(&"");
                let (service, message) = if let Some(colon_pos) = service_and_msg.find(':') {
                    let svc = &service_and_msg[..colon_pos];
                    let msg = service_and_msg[colon_pos + 1..].trim();
                    (svc.to_string(), msg.to_string())
                } else {
                    (service_and_msg.to_string(), String::new())
                };

                let upper = message.to_uppercase();
                let level = if upper.contains("ERROR") || upper.contains("FAIL") {
                    "ERROR"
                } else if upper.contains("WARN") {
                    "WARNING"
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

        if let Ok(content) = std::fs::read_to_string(grub_file) {
            for line in content.lines() {
                if line.starts_with("GRUB_") && !line.starts_with('#') {
                    if let Some(pos) = line.find('=') {
                        let key = line[..pos].to_string();
                        let mut value = line[pos + 1..].to_string();

                        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                            value = value[1..value.len() - 1].to_string();
                        }

                        configs.push(ConfigItem {
                            key,
                            value: value.clone(),
                            original_value: value,
                            description: "GRUB boot parameter".to_string(),
                        });
                    }
                }
            }
        }

        if let Some(sdboot_path) = find_systemd_boot_loader_conf() {
            if let Ok(content) = std::fs::read_to_string(&sdboot_path) {
                configs.extend(parse_systemd_boot_loader_conf(&content));
            }
        }

        if let Ok(cmdline) = std::fs::read_to_string("/etc/kernel/cmdline") {
            let val = cmdline.trim().to_string();
            if !val.is_empty() {
                configs.push(ConfigItem {
                    key: "KERNEL_CMDLINE".to_string(),
                    value: val.clone(),
                    original_value: val,
                    description: "/etc/kernel/cmdline boot args".to_string(),
                });
            }
        }

        let hostname_file = "/etc/hostname";
        if let Ok(hostname) = std::fs::read_to_string(hostname_file) {
            configs.push(ConfigItem {
                key: "hostname".to_string(),
                value: hostname.trim().to_string(),
                original_value: hostname.trim().to_string(),
                description: "System hostname".to_string(),
            });
        }

        let tz = ffi::read_timezone_native().or_else(|| {
            Command::new("timedatectl")
                .args(["show", "--value", "--property=Timezone"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        });
        if let Some(tz) = tz {
            configs.push(ConfigItem {
                key: "timezone".to_string(),
                value: tz.clone(),
                original_value: tz,
                description: "System timezone".to_string(),
            });
        }

        configs
    }

    pub fn get_sysctl_config(&self) -> Vec<ConfigItem> {
        const CURATED_SYSCTLS: &[(&str, &str)] = &[
            ("vm.swappiness", "Swap aggressiveness (0-200)"),
            ("vm.vfs_cache_pressure", "VFS inode/dentry cache reclaim bias"),
            ("vm.dirty_ratio", "Max dirty memory % before sync writeback"),
            ("vm.dirty_background_ratio", "Dirty memory % to start bg writeback"),
            ("vm.max_map_count", "Max memory map areas per process"),
            ("vm.overcommit_memory", "Memory overcommit policy (0/1/2)"),
            ("fs.file-max", "System-wide max open file handles"),
            ("fs.inotify.max_user_watches", "Max inotify watches per user"),
            ("net.ipv4.ip_forward", "IPv4 packet forwarding (0/1)"),
            ("net.ipv4.tcp_congestion_control", "Active TCP congestion control"),
            ("net.core.somaxconn", "Max listen socket backlog"),
            ("net.core.rmem_max", "Max socket receive buffer (bytes)"),
            ("net.core.wmem_max", "Max socket send buffer (bytes)"),
            ("kernel.sysrq", "Magic SysRq key bitmask"),
            ("kernel.perf_event_paranoid", "Perf event access level"),
            ("kernel.sched_autogroup_enabled", "Desktop scheduler autogroup (0/1)"),
        ];

        let mut items = Vec::new();
        for &(key, desc) in CURATED_SYSCTLS {
            let path = sysctl_key_to_proc_path(key);
            if let Ok(raw) = std::fs::read_to_string(&path) {
                let val = raw.trim().to_string();
                items.push(ConfigItem {
                    key: key.to_string(),
                    value: val.clone(),
                    original_value: val,
                    description: desc.to_string(),
                });
            }
        }
        items
    }

    pub fn apply_sysctl_changes(&self, changes: &[(String, String)]) -> Result<String, String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }
        if changes.is_empty() {
            return Ok(String::new());
        }

        let mut log = String::new();
        for (key, value) in changes {
            let proc_path = sysctl_key_to_proc_path(key);
            std::fs::write(&proc_path, format!("{}\n", value.trim()))
                .map_err(|e| format!("Failed writing {}: {}", proc_path.display(), e))?;
            log.push_str(&format!("[+] Applied live sysctl {} = {}\n", key, value.trim()));
        }

        let dropin_path = Path::new("/etc/sysctl.d/99-puls.conf");
        if dropin_path.exists() {
            let ts = crate::utils::current_formatted_time("%Y%m%d_%H%M%S");
            let bak = format!("/etc/sysctl.d/99-puls.conf.bak.{}", ts);
            let _ = std::fs::copy(dropin_path, &bak);
        }
        let existing = std::fs::read_to_string(dropin_path).unwrap_or_default();
        let merged = merge_sysctl_conf(&existing, changes);
        ffi::atomic_write_file(dropin_path, merged.as_bytes())
            .map_err(|e| format!("Failed persisting {}: {}", dropin_path.display(), e))?;
        log.push_str("[+] Persisted sysctl changes to /etc/sysctl.d/99-puls.conf.\n");

        Ok(log)
    }

    pub fn apply_config_changes(&self, changes: &[(String, String)]) -> Result<String, String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        let mut output_log = String::new();
        let mut grub_changes = Vec::new();
        let mut sdboot_changes = Vec::new();
        let mut cmdline_change: Option<String> = None;

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
            } else if let Some(sd_key) = key.strip_prefix("SDBOOT_") {
                sdboot_changes.push((sd_key.to_string(), value.clone()));
            } else if key == "KERNEL_CMDLINE" {
                cmdline_change = Some(value.clone());
            }
        }

        if !sdboot_changes.is_empty() {
            let sdboot_path = find_systemd_boot_loader_conf()
                .ok_or_else(|| "systemd-boot loader.conf not found".to_string())?;
            let ts = crate::utils::current_formatted_time("%Y%m%d_%H%M%S");
            let bak = format!("{}.bak.{}", sdboot_path.display(), ts);
            std::fs::copy(&sdboot_path, &bak)
                .map_err(|e| format!("Failed backing up {}: {}", sdboot_path.display(), e))?;
            output_log.push_str(&format!("[+] Created systemd-boot backup at {}.\n", bak));

            let content = std::fs::read_to_string(&sdboot_path)
                .map_err(|e| format!("Failed reading {}: {}", sdboot_path.display(), e))?;
            let updated = update_systemd_boot_loader_conf(&content, &sdboot_changes);
            ffi::atomic_write_file(&sdboot_path, updated.as_bytes())
                .map_err(|e| format!("Failed updating {}: {}", sdboot_path.display(), e))?;
            output_log.push_str(&format!("[+] Updated {}.\n", sdboot_path.display()));
        }

        if let Some(new_cmdline) = cmdline_change {
            let cmdline_path = Path::new("/etc/kernel/cmdline");
            let ts = crate::utils::current_formatted_time("%Y%m%d_%H%M%S");
            let bak = format!("/etc/kernel/cmdline.bak.{}", ts);
            let _ = std::fs::copy(cmdline_path, &bak);
            ffi::atomic_write_file(cmdline_path, format!("{}\n", new_cmdline.trim()).as_bytes())
                .map_err(|e| format!("Failed updating /etc/kernel/cmdline: {}", e))?;
            output_log.push_str("[+] Updated /etc/kernel/cmdline.\n");
        }

        if !grub_changes.is_empty() {
            let grub_file = "/etc/default/grub";
            let timestamp = crate::utils::current_formatted_time("%Y%m%d_%H%M%S");
            let backup_file = format!("{}.bak.{}", grub_file, timestamp);

            std::fs::copy(grub_file, &backup_file)
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

            ffi::atomic_write_file(Path::new(grub_file), new_content.as_bytes())
                .map_err(|e| format!("Failed to atomically write GRUB config: {}", e))?;

            output_log.push_str("[+] Successfully wrote changes to /etc/default/grub.\n");

            output_log.push_str("Compiling bootloader config...\n");
            let update_output = if Command::new("update-grub").output().is_ok() {
                let output = Command::new("update-grub")
                    .output()
                    .map_err(|e| format!("Failed to run update-grub: {}", e))?;
                format!("update-grub succeeded:\n{}", String::from_utf8_lossy(&output.stdout))
            } else if Command::new("grub-mkconfig").output().is_ok() {
                let output = Command::new("grub-mkconfig")
                    .args(["-o", "/boot/grub/grub.cfg"])
                    .output()
                    .map_err(|e| format!("Failed to run grub-mkconfig: {}", e))?;
                format!("grub-mkconfig succeeded:\n{}", String::from_utf8_lossy(&output.stdout))
            } else if Command::new("grub2-mkconfig").output().is_ok() {
                let output = Command::new("grub2-mkconfig")
                    .args(["-o", "/boot/grub2/grub.cfg"])
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

        if ffi::set_hostname_native(new_hostname).is_ok() {
            return Ok(());
        }

        let output = Command::new("hostnamectl")
            .args(["set-hostname", new_hostname])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn set_timezone(&self, timezone: &str) -> Result<(), String> {
        if !self.has_sudo {
            return Err("Insufficient privileges (root required)".to_string());
        }

        if ffi::set_timezone_native(timezone).is_ok() {
            return Ok(());
        }

        let output = Command::new("timedatectl")
            .args(["set-timezone", timezone])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn get_systemd_timers(&self) -> Vec<SystemTimerInfo> {
        let mut timers = Vec::new();
        if let Ok(output) = self
            .systemctl_cmd()
            .args(["list-timers", "--no-pager", "--no-legend", "--full"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(timer) = parse_systemd_timer_line(line) {
                    timers.push(timer);
                }
            }
        }
        timers
    }

    pub fn get_logged_in_users(&self) -> Vec<UserSessionInfo> {
        let native_sessions = ffi::read_user_sessions_native();
        if !native_sessions.is_empty() {
            return native_sessions;
        }

        let mut sessions = Vec::new();
        if let Ok(output) = Command::new("who").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(session) = parse_who_line(line) {
                    sessions.push(session);
                }
            }
        }
        sessions
    }
}

pub fn find_systemd_boot_loader_conf() -> Option<PathBuf> {
    for candidate in [
        "/boot/efi/loader/loader.conf",
        "/boot/loader/loader.conf",
        "/efi/loader/loader.conf",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn parse_systemd_boot_loader_conf(content: &str) -> Vec<ConfigItem> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            let val = v.trim().to_string();
            items.push(ConfigItem {
                key: format!("SDBOOT_{}", k),
                value: val.clone(),
                original_value: val,
                description: "systemd-boot loader.conf".to_string(),
            });
        }
    }
    items
}

pub fn update_systemd_boot_loader_conf(content: &str, changes: &[(String, String)]) -> String {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    for (key, value) in changes {
        let mut found = false;
        for line in &mut lines {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') {
                if let Some(first_tok) = trimmed.split_whitespace().next() {
                    if first_tok == key {
                        *line = format!("{} {}", key, value);
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            lines.push(format!("{} {}", key, value));
        }
    }
    lines.join("\n") + "\n"
}

pub fn sysctl_key_to_proc_path(key: &str) -> PathBuf {
    let rel = key.replace('.', "/");
    PathBuf::from("/proc/sys").join(rel)
}

pub fn merge_sysctl_conf(existing: &str, changes: &[(String, String)]) -> String {
    let mut lines: Vec<String> = if existing.is_empty() {
        vec!["# Managed by PULS".to_string()]
    } else {
        existing.lines().map(|s| s.to_string()).collect()
    };

    for (key, value) in changes {
        let mut found = false;
        for line in &mut lines {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        *line = format!("{} = {}", key, value.trim());
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            lines.push(format!("{} = {}", key, value.trim()));
        }
    }
    lines.join("\n") + "\n"
}

pub fn parse_journal_priority(code: &str) -> &'static str {
    match code.trim() {
        "0" => "EMERG",
        "1" => "ALERT",
        "2" => "CRIT",
        "3" => "ERROR",
        "4" => "WARNING",
        "5" => "NOTICE",
        "6" => "INFO",
        "7" => "DEBUG",
        _ => "INFO",
    }
}

pub fn parse_journal_json_line(line: &str) -> Option<LogEntry> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let val = JsonValue::parse(trimmed).ok()?;

    let timestamp = val
        .get("__REALTIME_TIMESTAMP")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .map(|usec| crate::utils::format_timestamp(usec / 1_000_000, "%b %d %H:%M:%S"))
        .unwrap_or_else(|| "unknown".to_string());

    let level = val
        .get("PRIORITY")
        .and_then(|v| v.as_str())
        .map(parse_journal_priority)
        .unwrap_or("INFO")
        .to_string();

    let service = val
        .get("SYSLOG_IDENTIFIER")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("_SYSTEMD_UNIT").and_then(|v| v.as_str()))
        .or_else(|| val.get("_COMM").and_then(|v| v.as_str()))
        .unwrap_or("systemd")
        .to_string();

    let message = match val.get("MESSAGE") {
        Some(JsonValue::String(s)) => s.clone(),
        Some(JsonValue::Array(arr)) => {
            let bytes: Vec<u8> = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            String::from_utf8_lossy(&bytes).into_owned()
        }
        _ => String::new(),
    };

    Some(LogEntry {
        timestamp,
        level,
        service,
        message,
    })
}

pub fn parse_boot_blame(content: &str) -> Vec<BootBlameEntry> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() >= 2 {
            let unit = tokens[tokens.len() - 1].to_string();
            let duration = tokens[..tokens.len() - 1].join(" ");
            entries.push(BootBlameEntry { duration, unit });
        }
    }
    entries
}

pub fn parse_systemd_timer_line(line: &str) -> Option<SystemTimerInfo> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }

    let activates = tokens[tokens.len() - 1].to_string();
    let unit = tokens[tokens.len() - 2].to_string();
    if !unit.ends_with(".timer") {
        return None;
    }

    let body = &tokens[..tokens.len() - 2];
    let (next, left, rest) = if let Some(left_idx) = body.iter().position(|&t| t == "left") {
        let (next_tokens, left_tokens) = if left_idx >= 4 && body[1].contains('-') {
            let split_pt = if left_idx > 4 && (body[3].starts_with('+') || body[3].starts_with('-')) { 4 }
            else if left_idx > 3 && (body[4].starts_with('+') || body[4].starts_with('-')) { 5 }
            else { 3.min(left_idx.saturating_sub(1)) };
            (&body[..split_pt], &body[split_pt..=left_idx])
        } else if left_idx > 0 {
            (&body[..left_idx.saturating_sub(1)], &body[left_idx.saturating_sub(1)..=left_idx])
        } else {
            (&body[..left_idx], &body[left_idx..=left_idx])
        };
        (next_tokens.join(" "), left_tokens.join(" "), &body[left_idx + 1..])
    } else if body.first() == Some(&"n/a") {
        let end_na = body.iter().take_while(|&&t| t == "n/a").count();
        ("n/a".to_string(), "n/a".to_string(), &body[end_na..])
    } else {
        ("—".to_string(), "—".to_string(), body)
    };

    let (last, passed) = if let Some(ago_idx) = rest.iter().position(|&t| t == "ago") {
        let split_pt = if ago_idx >= 4 && rest[1].contains('-') {
            if ago_idx > 4 && (rest[4].starts_with('+') || rest[4].starts_with('-')) { 5 }
            else { 4 }
        } else if ago_idx > 0 {
            ago_idx.saturating_sub(1)
        } else {
            ago_idx
        };
        (rest[..split_pt].join(" "), rest[split_pt..=ago_idx].join(" "))
    } else if rest.first() == Some(&"n/a") {
        ("n/a".to_string(), "n/a".to_string())
    } else {
        (rest.join(" "), String::new())
    };

    Some(SystemTimerInfo {
        next,
        left,
        last,
        passed,
        unit,
        activates,
    })
}

pub fn parse_who_line(line: &str) -> Option<UserSessionInfo> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }

    let user = tokens[0].to_string();
    let line_tty = tokens[1].to_string();
    let login_time = if tokens.len() >= 4 && tokens[2].contains('-') {
        format!("{} {}", tokens[2], tokens[3])
    } else {
        tokens[2].to_string()
    };

    let host = if let Some(last) = tokens.last() {
        if last.starts_with('(') && last.ends_with(')') {
            last.trim_start_matches('(').trim_end_matches(')').to_string()
        } else if tokens.len() > 4 {
            tokens[4..].join(" ")
        } else {
            "local".to_string()
        }
    } else {
        "local".to_string()
    };

    Some(UserSessionInfo {
        user,
        line: line_tty,
        login_time,
        host,
    })
}

impl Default for SystemManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_systemd_timer_line() {
        let line = "Sat 2026-09-19 14:07:16 +03 6s left Sat 2026-09-19 13:25:42 +03 41min ago fwupd-refresh.timer fwupd-refresh.service";
        let timer = parse_systemd_timer_line(line).expect("parse timer failed");
        assert_eq!(timer.unit, "fwupd-refresh.timer");
        assert_eq!(timer.activates, "fwupd-refresh.service");
        assert_eq!(timer.left, "6s left");
        assert_eq!(timer.passed, "41min ago");
    }

    #[test]
    fn test_parse_systemd_timer_line_na() {
        let line = "Sun 2026-09-20 00:00:00 +03 9h left n/a n/a dpkg-db-backup.timer dpkg-db-backup.service";
        let timer = parse_systemd_timer_line(line).expect("parse timer failed");
        assert_eq!(timer.unit, "dpkg-db-backup.timer");
        assert_eq!(timer.activates, "dpkg-db-backup.service");
        assert_eq!(timer.left, "9h left");
        assert_eq!(timer.last, "n/a");
        assert_eq!(timer.passed, "n/a");
    }

    #[test]
    fn test_parse_who_line() {
        let line = "word-sys :0 2026-09-19 12:43 ? 2506 (:0)";
        let session = parse_who_line(line).expect("parse who failed");
        assert_eq!(session.user, "word-sys");
        assert_eq!(session.line, ":0");
        assert_eq!(session.login_time, "2026-09-19 12:43");
        assert_eq!(session.host, ":0");

        let line2 = "alice pts/1 2026-09-19 10:15 (192.168.1.100)";
        let session2 = parse_who_line(line2).expect("parse who failed");
        assert_eq!(session2.user, "alice");
        assert_eq!(session2.line, "pts/1");
        assert_eq!(session2.login_time, "2026-09-19 10:15");
        assert_eq!(session2.host, "192.168.1.100");
    }

    #[test]
    fn test_parse_journal_json_and_boot_blame() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1700000000000000","PRIORITY":"3","SYSLOG_IDENTIFIER":"sshd","MESSAGE":"Failed password for root"}"#;
        let entry = parse_journal_json_line(line).expect("parse journal json failed");
        assert_eq!(entry.level, "ERROR");
        assert_eq!(entry.service, "sshd");
        assert_eq!(entry.message, "Failed password for root");

        let warn_line = r#"{"__REALTIME_TIMESTAMP":"1700000000000000","PRIORITY":"4","_SYSTEMD_UNIT":"docker.service","MESSAGE":"Daemon slow"}"#;
        let warn_entry = parse_journal_json_line(warn_line).expect("parse warn json failed");
        assert_eq!(warn_entry.level, "WARNING");
        assert_eq!(warn_entry.service, "docker.service");

        let blame_txt = "  2.415s NetworkManager-wait-online.service\n  842ms snapd.service\n  120ms systemd-udev-trigger.service\n";
        let blame = parse_boot_blame(blame_txt);
        assert_eq!(blame.len(), 3);
        assert_eq!(blame[0].duration, "2.415s");
        assert_eq!(blame[0].unit, "NetworkManager-wait-online.service");
    }

    #[test]
    fn test_systemd_boot_and_sysctl_helpers() {
        let loader = "# systemd-boot\ndefault arch.conf\ntimeout 3\nconsole-mode max\n";
        let items = parse_systemd_boot_loader_conf(loader);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].key, "SDBOOT_default");
        assert_eq!(items[1].value, "3");

        let updated = update_systemd_boot_loader_conf(
            loader,
            &[("timeout".to_string(), "5".to_string()), ("editor".to_string(), "no".to_string())],
        );
        assert!(updated.contains("timeout 5\n"));
        assert!(updated.contains("editor no\n"));

        assert_eq!(
            sysctl_key_to_proc_path("vm.swappiness"),
            PathBuf::from("/proc/sys/vm/swappiness")
        );
        let merged = merge_sysctl_conf(
            "vm.swappiness = 60\n",
            &[
                ("vm.swappiness".to_string(), "10".to_string()),
                ("net.ipv4.ip_forward".to_string(), "1".to_string()),
            ],
        );
        assert!(merged.contains("vm.swappiness = 10\n"));
        assert!(merged.contains("net.ipv4.ip_forward = 1\n"));
    }
}