mod types;
mod utils;
mod config;
mod monitors;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Terminal};
use tokio::time::sleep;

use clap::Parser;
use crate::config::{Cli, AppConfig};
use crate::monitors::DataCollector;
use crate::types::{AppState, ProcessSortBy, AppMessage};
use crate::ui::render_ui;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    let config = AppConfig::from(cli);
    
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Initialize application state
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let data_collector = Arc::new(Mutex::new(DataCollector::new(config.clone())));
    
    // Get initial system information
    let system_info = {
        let collector = data_collector.lock();
        collector.get_system_info()
    };
    
    // Set initial system info
    {
        let mut state = app_state.lock();
        state.system_info = system_info;
        
        // Add performance mode info
        if config.safe_mode {
            state.system_info.push(("Mode".to_string(), "Safe Mode".to_string()));
        }
    }
    
    // Spawn data collection task
    let app_state_clone = app_state.clone();
    let data_collector_clone = data_collector.clone();
    let config_clone = config.clone();
    
    tokio::spawn(async move {
        data_collection_loop(app_state_clone, data_collector_clone, config_clone).await;
    });
    
    // Main UI loop
    let result = ui_loop(&mut terminal, app_state, &config).await;
    
    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    
    // Print any final messages
    if let Err(ref e) = result {
        eprintln!("Application error: {}", e);
    }
    
    result
}

/// Main UI event loop with smooth 60 FPS rendering
async fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: Arc<Mutex<AppState>>,
    config: &AppConfig,
) -> io::Result<()> {
    let ui_refresh_interval = Duration::from_millis(config.ui_refresh_rate_ms());
    let mut last_render = Instant::now();
    
    loop {
        let now = Instant::now();
        
        // Handle input events (non-blocking)
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                let should_quit = handle_key_event(key, &app_state)?;
                if should_quit {
                    return Ok(());
                }
            }
        }
        
        // Render UI at target framerate
        if now.duration_since(last_render) >= ui_refresh_interval {
            {
                let mut state = app_state.lock();
                terminal.draw(|f| render_ui(f, &mut state, config.safe_mode))?;
            }
            last_render = now;
        }
        
        // Small sleep to prevent excessive CPU usage
        sleep(Duration::from_millis(1)).await;
    }
}

/// Handle keyboard input events
fn handle_key_event(
    key: crossterm::event::KeyEvent,
    app_state: &Arc<Mutex<AppState>>,
) -> io::Result<bool> {
    let mut state = app_state.lock();
    
    match key.code {
        // Quit application
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            return Ok(true);
        }
        
        // Toggle pause
        KeyCode::Char('p') | KeyCode::Char('P') => {
            state.paused = !state.paused;
        }
        
        // Tab navigation
        KeyCode::Tab => {
            state.active_tab = (state.active_tab + 1) % 7;
        }
        KeyCode::BackTab => {
            state.active_tab = (state.active_tab + 6) % 7;
        }
        
        // Direct tab access
        KeyCode::Char('1') => state.active_tab = 0,
        KeyCode::Char('2') => state.active_tab = 1,
        KeyCode::Char('3') => state.active_tab = 2,
        KeyCode::Char('4') => state.active_tab = 3,
        KeyCode::Char('5') => state.active_tab = 4,
        KeyCode::Char('6') => state.active_tab = 5,
        KeyCode::Char('7') => state.active_tab = 6,
        
        // Process list navigation (Dashboard tab)
        KeyCode::Down if state.active_tab == 0 => {
            handle_process_navigation(&mut state, true);
        }
        KeyCode::Up if state.active_tab == 0 => {
            handle_process_navigation(&mut state, false);
        }
        
        // Select process for detailed view
        KeyCode::Enter if state.active_tab == 0 => {
            if let Some(selected_index) = state.process_table_state.selected() {
                if let Some(process) = state.dynamic_data.processes.get(selected_index) {
                    if let Ok(pid_val) = process.pid.parse::<usize>() {
                        state.selected_pid = Some(sysinfo::Pid::from(pid_val));
                        state.active_tab = 1; // Switch to process detail tab
                    }
                }
            }
        }
        
        // Sorting controls (when in dashboard)
        KeyCode::Char('c') if state.active_tab == 0 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Cpu;
            state.sort_ascending = !state.sort_ascending;
        }
        KeyCode::Char('m') if state.active_tab == 0 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Memory;
            state.sort_ascending = !state.sort_ascending;
        }
        KeyCode::Char('n') if state.active_tab == 0 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Name;
            state.sort_ascending = !state.sort_ascending;
        }
        
        // Toggle system processes
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.show_system_processes = !state.show_system_processes;
        }
        
        // Help (could show a help popup in future)
        KeyCode::Char('h') | KeyCode::F1 => {
            // TODO: Implement help popup
        }
        
        _ => {}
    }
    
    Ok(false)
}

/// Handle process list navigation
fn handle_process_navigation(state: &mut AppState, down: bool) {
    let processes = &state.dynamic_data.processes;
    if processes.is_empty() {
        return;
    }
    
    let current = state.process_table_state.selected().unwrap_or(0);
    let new_index = if down {
        if current >= processes.len() - 1 { 0 } else { current + 1 }
    } else {
        if current == 0 { processes.len() - 1 } else { current - 1 }
    };
    
    state.process_table_state.select(Some(new_index));
}

/// Background data collection loop
async fn data_collection_loop(
    app_state: Arc<Mutex<AppState>>,
    data_collector: Arc<Mutex<DataCollector>>,
    config: AppConfig,
) {
    let mut interval = tokio::time::interval(config.get_collection_sleep_duration());
    let mut prev_global_usage = types::GlobalUsage::default();
    
    loop {
        interval.tick().await;
        
        // Skip collection if paused
        let is_paused = {
            let state = app_state.lock();
            state.paused
        };
        
        if is_paused {
            continue;
        }
        
        let collection_start = Instant::now();
        
        // Get current state for collection parameters
        let (selected_pid, show_system_processes, filter_text) = {
            let state = app_state.lock();
            (
                state.selected_pid,
                state.show_system_processes,
                state.filter_text.clone(),
            )
        };
        
        // Collect data
        let new_data = {
            let mut collector = data_collector.lock();
            collector.collect_data(
                selected_pid,
                show_system_processes,
                &filter_text,
                prev_global_usage.clone(),
            ).await
        };
        
        prev_global_usage = new_data.global_usage.clone();
        
        // Update application state
        {
            let mut state = app_state.lock();
            state.dynamic_data = new_data;
            
            // Ensure process table has a selection
            if state.process_table_state.selected().is_none() && !state.dynamic_data.processes.is_empty() {
                state.process_table_state.select(Some(0));
            }
        }
        
        let collection_duration = collection_start.elapsed();
        
        // Log slow collections for debugging
        if collection_duration > Duration::from_millis(config.refresh_rate_ms / 2) {
            eprintln!("Slow data collection: {:?}", collection_duration);
        }
        
        // Adaptive sleep - if collection took too long, skip sleep to maintain target rate
        let remaining_time = config.get_collection_sleep_duration().saturating_sub(collection_duration);
        if remaining_time > Duration::from_millis(10) {
            sleep(remaining_time).await;
        }
    }
}

/// Custom error type for the application
#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Config(String),
    Monitor(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO Error: {}", e),
            AppError::Config(e) => write!(f, "Configuration Error: {}", e),
            AppError::Monitor(e) => write!(f, "Monitoring Error: {}", e),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

/// Signal handler for graceful shutdown
#[cfg(unix)]
fn setup_signal_handlers() -> Result<(), Box<dyn std::error::Error>> {
    use signal_hook::{consts::SIGTERM, iterator::Signals};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    
    let term = Arc::new(AtomicBool::new(false));
    let mut signals = Signals::new(&[SIGTERM])?;
    
    let term_clone = Arc::clone(&term);
    std::thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGTERM => {
                    term_clone.store(true, Ordering::Relaxed);
                }
                _ => unreachable!(),
            }
        }
    });
    
    Ok(())
}

/// Check system requirements and capabilities
fn check_system_requirements() -> Result<(), AppError> {
    // Check if we're running in a proper terminal
    if !atty::is(atty::Stream::Stdout) {
        return Err(AppError::Config(
            "PULS requires a terminal environment".to_string()
        ));
    }
    
    // Check minimum terminal size
    if let Ok((width, height)) = crossterm::terminal::size() {
        if width < 80 || height < 24 {
            eprintln!("Warning: Terminal size {}x{} is smaller than recommended 80x24", width, height);
        }
    }
    
    Ok(())
}

/// Initialize logging (if enabled)
fn init_logging(verbose: bool) -> Result<(), AppError> {
    if verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_display() {
        let io_error = AppError::Io(io::Error::new(io::ErrorKind::NotFound, "test"));
        assert!(format!("{}", io_error).contains("IO Error"));
        
        let config_error = AppError::Config("test config error".to_string());
        assert!(format!("{}", config_error).contains("Configuration Error"));
        
        let monitor_error = AppError::Monitor("test monitor error".to_string());
        assert!(format!("{}", monitor_error).contains("Monitoring Error"));
    }
}