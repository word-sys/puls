use std::fs::OpenOptions;
use std::io::Write;

pub fn log_error(error: &str) {
    let log_file = "puls_error.log";
    
    let timestamp = crate::utils::current_formatted_time("%Y-%m-%d %H:%M:%S");
    let message = format!("[{}] {}\n", timestamp, error);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file) 
    {
        let _ = file.write_all(message.as_bytes());
    }
}