#![allow(dead_code)]

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn format_rate(bytes_per_sec: u64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    const THRESHOLD: f64 = 1000.0;
    
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }
    
    let mut rate = bytes_per_sec as f64;
    let mut unit_index = 0;
    
    while rate >= THRESHOLD && unit_index < UNITS.len() - 1 {
        rate /= THRESHOLD;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes_per_sec, UNITS[unit_index])
    } else {
        format!("{:.1} {}", rate, UNITS[unit_index])
    }
}

pub fn format_frequency(hz: u64) -> String {
    let hz_value = hz * 1_000_000;
    
    if hz_value >= 1_000_000_000 {
        format!("{:.2} GHz", hz_value as f64 / 1_000_000_000.0)
    } else if hz_value >= 1_000_000 {
        format!("{:.0} MHz", hz_value as f64 / 1_000_000.0)
    } else if hz_value >= 1_000 {
        format!("{:.0} KHz", hz_value as f64 / 1_000.0)
    } else {
        format!("{} Hz", hz_value)
    }
}

pub fn format_frequency_hz(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.2} GHz", hz as f64 / 1_000_000_000.0)
    } else if hz >= 1_000_000 {
        format!("{:.0} MHz", hz as f64 / 1_000_000.0)
    } else if hz >= 1_000 {
        format!("{:.0} KHz", hz as f64 / 1_000.0)
    } else {
        format!("{} Hz", hz)
    }
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn format_percentage(value: f32) -> String {
    format!("{:.1}%", value)
}

pub fn format_temperature(celsius: f32) -> String {
    format!("{:.1}°C", celsius)
}

pub fn format_temperature_with_status(celsius: f32) -> String {
    let status = match celsius {
        x if x >= 90.0 => "HOT",
        x if x >= 75.0 => "WARM",
        x if x >= 60.0 => "NORMAL",
        _ => "COOL",
    };
    format!("{:.1}°C {}", celsius, status)
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn safe_percentage(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0) as f32
    }
}

pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

pub fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

pub fn get_usage_color(percentage: f32) -> ratatui::style::Color {
    use ratatui::style::Color;
    
    if percentage >= 90.0 {
        Color::Red
    } else if percentage >= 75.0 {
        Color::Yellow
    } else if percentage >= 50.0 {
        Color::Cyan
    } else {
        Color::Green
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

pub fn is_system_process(name: &str) -> bool {
    const SYSTEM_PROCESSES: &[&str] = &[
        "kthreadd", "migration", "rcu_", "watchdog", "systemd",
        "kernel", "kworker", "ksoftirqd", "init", "swapper",
        "[", "dbus", "NetworkManager", "systemd-"
    ];
    
    SYSTEM_PROCESSES.iter().any(|&sys_proc| name.starts_with(sys_proc))
}

pub fn update_history<T: Clone>(history: &mut VecDeque<T>, new_value: T, max_size: usize) {
    history.push_back(new_value);
    while history.len() > max_size {
        history.pop_front();
    }
}

pub fn calculate_rate(current: u64, previous: u64, elapsed_secs: f64) -> u64 {
    if elapsed_secs <= 0.0 {
        return 0;
    }
    
    let diff = current.saturating_sub(previous);
    (diff as f64 / elapsed_secs) as u64
}

pub fn matches_filter(text: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    
    let text_lower = text.to_lowercase();
    let filter_lower = filter.to_lowercase();
    
    text_lower.contains(&filter_lower)
}

pub fn get_top_processes(processes: &[crate::types::ProcessInfo], top_n: usize) -> Vec<String> {
    let mut sorted = processes.to_vec();
    sorted.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
    
    sorted.iter()
        .take(top_n)
        .map(|p| format!("{}: {:.1}%", p.name, p.cpu))
        .collect()
}

pub fn get_top_memory_consumers(processes: &[crate::types::ProcessInfo], top_n: usize) -> Vec<String> {
    let mut sorted = processes.to_vec();
    sorted.sort_by(|a, b| b.mem.cmp(&a.mem));
    
    sorted.iter()
        .take(top_n)
        .map(|p| format!("{}: {}", p.name, p.mem_display))
        .collect()
}

pub fn count_process_states(processes: &[crate::types::ProcessInfo]) -> (usize, usize, usize, usize) {
    let mut running = 0;
    let mut sleeping = 0;
    let mut zombie = 0;
    let mut other = 0;
    
    for process in processes {
        match process.status.to_lowercase().as_str() {
            "running" | "r" => running += 1,
            "sleeping" | "s" => sleeping += 1,
            "zombie" | "z" => zombie += 1,
            _ => other += 1,
        }
    }
    
    (running, sleeping, zombie, other)
}

pub fn estimate_memory_per_core(mem_used: u64, cpu_cores: usize) -> u64 {
    if cpu_cores > 0 {
        mem_used / cpu_cores as u64
    } else {
        mem_used
    }
}

pub fn get_cpu_efficiency(cpu_percent: f32, load_avg: f64, cpu_cores: usize) -> String {
    if cpu_percent < 5.0 && load_avg < 0.2 * cpu_cores as f64 {
        return "IDLE".to_string();
    }

    let load_per_core = if cpu_cores > 0 {
        load_avg / cpu_cores as f64
    } else {
        load_avg
    };

    let efficiency = if load_per_core > 0.0 {
        ((cpu_percent as f64 / 100.0) / load_per_core * 100.0).min(100.0)
    } else {
        100.0
    };
    
    match efficiency {
        x if x >= 90.0 => "OPTIMAL".to_string(),
        x if x >= 70.0 => "GOOD".to_string(),
        x if x >= 50.0 => "FAIR".to_string(),
        _ => "POOR".to_string(),
    }
}

pub fn estimate_memory_availability(mem_used: u64, mem_total: u64) -> (u64, String) {
    let available = mem_total.saturating_sub(mem_used);
    let percent_free = if mem_total > 0 {
        (available as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let level = match percent_free {
        x if x >= 40.0 => "COMFORTABLE",
        x if x >= 20.0 => "MODERATE",
        x if x >= 10.0 => "TIGHT",
        _ => "CRITICAL",
    };
    
    (available, level.to_string())
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn format_load_average(load1: f64, load5: f64, load15: f64) -> String {
    format!("{:.2} {:.2} {:.2}", load1, load5, load15)
}

pub fn get_system_health(load_avg: f64, cpu_cores: usize, mem_used: u64, mem_total: u64) -> (String, String) {
    let load_per_core = if cpu_cores > 0 {
        load_avg / cpu_cores as f64
    } else {
        0.0
    };
    
    let mem_percent = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let load_status = match load_per_core {
        x if x >= 2.0 => ("CRITICAL", "red"),
        x if x >= 1.5 => ("OVERLOAD", "yellow"),
        x if x >= 1.0 => ("HIGH", "yellow"),
        x if x >= 0.5 => ("NORMAL", "green"),
        _ => ("IDLE", "green"),
    };
    
    let mem_status = match mem_percent {
        x if x >= 90.0 => ("CRITICAL", "red"),
        x if x >= 80.0 => ("HIGH", "yellow"),
        x if x >= 60.0 => ("MODERATE", "cyan"),
        _ => ("HEALTHY", "green"),
    };
    
    let status = format!("[{}/{}]", load_status.0, mem_status.0);
    (status, format!("{}", load_per_core))
}

pub fn get_memory_breakdown(mem_available: u64, mem_total: u64) -> (u64, u64) {
    let mem_used = mem_total.saturating_sub(mem_available);
    (mem_used, mem_available)
}

use std::os::raw::{c_int, c_long, c_char};
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

extern "C" {
    pub fn localtime(timep: *const i64) -> *mut tm;
    pub fn strftime(s: *mut c_char, max: usize, format: *const c_char, tm: *const tm) -> usize;
    pub fn getuid() -> u32;
}

pub fn format_timestamp(timestamp: i64, format_str: &str) -> String {
    unsafe {
        let t_ptr = localtime(&timestamp);
        if t_ptr.is_null() {
            return "unknown time".to_string();
        }
        let mut buf = [0u8; 128];
        let c_fmt = std::ffi::CString::new(format_str).unwrap_or_default();
        let len = strftime(
            buf.as_mut_ptr() as *mut std::os::raw::c_char,
            buf.len(),
            c_fmt.as_ptr(),
            t_ptr,
        );
        if len > 0 {
            String::from_utf8_lossy(&buf[..len]).into_owned()
        } else {
            "unknown time".to_string()
        }
    }
}

pub fn current_formatted_time(format_str: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_timestamp(now as i64, format_str)
}

pub fn get_current_uid() -> u32 {
    unsafe { getuid() }
}

#[derive(Clone)]
pub struct UsersCache {
    users: HashMap<u32, String>,
}

impl UsersCache {
    pub fn new() -> Self {
        let mut users = HashMap::new();
        if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
            for line in content.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let username = parts[0].to_string();
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        users.insert(uid, username);
                    }
                }
            }
        }
        UsersCache { users }
    }

    pub fn get_user_by_uid(&self, uid: u32) -> Option<String> {
        self.users.get(&uid).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(1048576), "1.0 MiB");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(format_rate(0), "0 B/s");
        assert_eq!(format_rate(500), "500 B/s");
        assert_eq!(format_rate(1000), "1.0 KB/s");
        assert_eq!(format_rate(1500), "1.5 KB/s");
    }

    #[test]
    fn test_safe_percentage() {
        assert_eq!(safe_percentage(50, 100), 50.0);
        assert_eq!(safe_percentage(0, 0), 0.0);
        assert_eq!(safe_percentage(100, 0), 0.0);
    }

    #[test]
    fn test_is_system_process() {
        assert!(is_system_process("kworker/0:1"));
        assert!(is_system_process("systemd-logind"));
        assert!(!is_system_process("firefox"));
        assert!(!is_system_process("puls"));
    }
}