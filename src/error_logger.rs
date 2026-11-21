use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_RING_ENTRIES: usize = 200;

static VERBOSE_LOGGING: AtomicBool = AtomicBool::new(false);
static LOG_RING: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

fn ring_buffer() -> &'static Mutex<VecDeque<String>> {
    LOG_RING.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_RING_ENTRIES)))
}

pub fn init(verbose: bool) {
    VERBOSE_LOGGING.store(verbose, Ordering::Relaxed);
}

pub fn log_event(level: &str, message: &str) {
    let timestamp = crate::utils::current_formatted_time("%Y-%m-%d %H:%M:%S");
    let formatted = format!("[{}] [{}] {}", timestamp, level, message);

    if let Ok(mut ring) = ring_buffer().lock() {
        if ring.len() >= MAX_RING_ENTRIES {
            ring.pop_front();
        }
        ring.push_back(formatted.clone());
    }

    if VERBOSE_LOGGING.load(Ordering::Relaxed) {
        eprintln!("{}", formatted);
    }

    if level == "ERROR" {
        write_to_error_file(&formatted);
    }
}

pub fn log_error(error: &str) {
    log_event("ERROR", error);
}

fn write_to_error_file(formatted_line: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("puls_error.log")
    {
        let _ = writeln!(file, "{}", formatted_line);
    }
}

#[allow(dead_code)]
pub fn get_recent_logs() -> Vec<String> {
    ring_buffer()
        .lock()
        .map(|r| r.iter().cloned().collect())
        .unwrap_or_default()
}

#[macro_export]
macro_rules! puls_error {
    ($($arg:tt)*) => {
        $crate::error_logger::log_event("ERROR", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! puls_warn {
    ($($arg:tt)*) => {
        $crate::error_logger::log_event("WARN", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! puls_info {
    ($($arg:tt)*) => {
        $crate::error_logger::log_event("INFO", &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_logging_and_cap() {
        init(false);
        for i in 0..250 {
            crate::puls_warn!("test warning {}", i);
        }
        let logs = get_recent_logs();
        assert!(logs.len() <= MAX_RING_ENTRIES);
        assert!(logs.last().unwrap().contains("test warning 249"));
        assert!(logs.last().unwrap().contains("[WARN]"));
    }
}
