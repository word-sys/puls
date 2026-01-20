use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

pub fn log_error(error: &str) {
    let log_file = "puls_error.log";
    
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let message = format!("[{}] {}\n", timestamp, error);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file) 
    {
        let _ = file.write_all(message.as_bytes());
    }
}