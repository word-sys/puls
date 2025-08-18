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

pub fn format_temperature(celsius: f32) -> String {
    format!("{:.1}°C", celsius)
}

pub fn matches_filter(text: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    
    let text_lower = text.to_lowercase();
    let filter_lower = filter.to_lowercase();
    
    text_lower.contains(&filter_lower)
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