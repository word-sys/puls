mod types;
mod utils;
mod config;
mod monitors;
mod ui;
mod language;
mod system_service;
mod error_logger;

use crate::types::{AppState, ProcessSortBy, ServiceInfo, LogEntry, ConfigItem, BootInfo};
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
use crate::config::{Cli};
use crate::monitors::DataCollector;
use crate::types::AppConfig;
use crate::ui::render_ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = AppConfig::from(cli);
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let data_collector = Arc::new(Mutex::new(DataCollector::new(config.clone())));
    
    let system_info = {
        let collector = data_collector.lock();
        collector.get_system_info()
    };
    
    {
        let mut state = app_state.lock();
        state.system_info = system_info;
        
        if config.safe_mode {
            state.system_info.push(("Mode".to_string(), "Safe Mode".to_string()));
        }
        
        let sys_mgr = system_service::SystemManager::new();
        state.has_sudo = sys_mgr.has_sudo_privileges();
        
        state.services = sys_mgr.get_services();
        if !state.services.is_empty() {
            state.services_table_state.select(Some(0));
        }
        
        state.logs = sys_mgr.get_logs(50, None, None);
        if !state.logs.is_empty() {
            state.logs_table_state.select(Some(0));
        }

        state.config_items = sys_mgr.get_grub_config();
        if !state.config_items.is_empty() {
            state.config_table_state.select(Some(0));
        }
        
        state.boots = sys_mgr.get_boots();
        if !state.boots.is_empty() {
            state.current_boot_idx = 0;
        }
    }
    
    let local = tokio::task::LocalSet::new();

    let result = local.run_until(async {
        let app_state_clone = app_state.clone();
        let data_collector_clone = data_collector.clone();
        let config_clone = config.clone();
        tokio::task::spawn_local(async move {
            data_collection_loop(app_state_clone, data_collector_clone, config_clone).await;
        });

        ui_loop(&mut terminal, app_state, &config).await
    }).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        eprintln!("Application error: {}", e);
        crate::error_logger::log_error(&e.to_string());
    }

    result.map_err(|e| e.into())
}

async fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: Arc<Mutex<AppState>>,
    config: &AppConfig,
) -> io::Result<()> {
    let ui_refresh_interval = Duration::from_millis(config.ui_refresh_rate_ms());
    let mut last_render = Instant::now();
    
    loop {
        let now = Instant::now();
        
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                let should_quit = handle_key_event(key, &app_state)?;
                if should_quit {
                    return Ok(());
                }
            }
        }
        
        if now.duration_since(last_render) >= ui_refresh_interval {
            {
                let mut state = app_state.lock();
                let translator = crate::language::Translator::new(config.language);
                terminal.draw(|f| render_ui(f, &mut state, config.safe_mode, &translator))?;
            }
            last_render = now;
        }
        
        sleep(Duration::from_millis(1)).await;
    }
}

fn handle_key_event(
    key: crossterm::event::KeyEvent,
    app_state: &Arc<Mutex<AppState>>,
) -> io::Result<bool> {
    let mut state = app_state.lock();
    
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            if state.service_status_modal.is_some() {
                 state.service_status_modal = None;
                 return Ok(false);
            }
            if state.editing_filter {
                state.editing_filter = false;
                state.edit_buffer.clear();
                return Ok(false);
            }
            if state.editing_service.is_some() || state.editing_config.is_some() {
                state.editing_service = None;
                state.editing_config = None;
                state.edit_buffer.clear();
                return Ok(false);
            }
            return Ok(true);
        }
        
        KeyCode::Char('l') if state.active_tab == 8 && state.service_status_modal.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    let sys_mgr = system_service::SystemManager::new();
                    let status = sys_mgr.get_service_status(&service.name);
                    state.service_status_modal = Some((service.name.clone(), status));
                }
            }
        }

        KeyCode::Char('/') if state.active_tab == 9 && !state.editing_filter => {
             state.editing_filter = true;
             state.edit_buffer = state.log_filter.clone();
        }

        KeyCode::Enter if state.editing_filter => {
             state.log_filter = state.edit_buffer.clone();
             state.editing_filter = false;
             state.edit_buffer.clear();
             let sys_mgr = system_service::SystemManager::new();
             state.logs = sys_mgr.get_logs(50, Some(&state.log_filter), None);
             state.logs_table_state.select(Some(0));
        }

        KeyCode::Char(c) if state.editing_filter => {
            state.edit_buffer.push(c);
        }

        KeyCode::Backspace if state.editing_filter => {
            state.edit_buffer.pop();
        }

        KeyCode::Char('>') | KeyCode::Right if state.active_tab == 9 && !state.editing_filter => {
            if !state.boots.is_empty() {
                if state.current_boot_idx > 0 {
                    state.current_boot_idx -= 1;
                    let sys_mgr = system_service::SystemManager::new();
                    let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
                    let filter = if state.log_filter.is_empty() { None } else { Some(state.log_filter.as_str()) };
                    state.logs = sys_mgr.get_logs(50, filter, boot_id);
                    state.logs_table_state.select(Some(0));
                }
            }
        }

        KeyCode::Char('<') | KeyCode::Left if state.active_tab == 9 && !state.editing_filter => {
            if !state.boots.is_empty() {
                if state.current_boot_idx < state.boots.len() - 1 {
                    state.current_boot_idx += 1;
                    let sys_mgr = system_service::SystemManager::new();
                    let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
                    let filter = if state.log_filter.is_empty() { None } else { Some(state.log_filter.as_str()) };
                    state.logs = sys_mgr.get_logs(50, filter, boot_id);
                    state.logs_table_state.select(Some(0));
                }
            }
        }

        KeyCode::Char('p') | KeyCode::Char('P') => {
            state.paused = !state.paused;
        }
        
        KeyCode::Tab => {
            state.active_tab = (state.active_tab + 1) % 12;
        }
        KeyCode::BackTab => {
            state.active_tab = (state.active_tab + 11) % 12;
        }
        
        KeyCode::Char('1') => state.active_tab = 0,
        KeyCode::Char('2') => state.active_tab = 1,
        KeyCode::Char('3') => state.active_tab = 2,
        KeyCode::Char('4') => state.active_tab = 3,
        KeyCode::Char('5') => state.active_tab = 4,
        KeyCode::Char('6') => state.active_tab = 5,
        KeyCode::Char('7') => state.active_tab = 6,
        KeyCode::Char('8') => state.active_tab = 7,
        KeyCode::Char('9') => state.active_tab = 8,
        KeyCode::Char('0') => state.active_tab = 9,
        KeyCode::Char('-') => state.active_tab = 10,
        KeyCode::Char('=') => state.active_tab = 11,
        
        KeyCode::Down if state.active_tab == 0 => {
            handle_process_navigation(&mut state, true);
        }
        KeyCode::Up if state.active_tab == 0 => {
            handle_process_navigation(&mut state, false);
        }
        
        KeyCode::Down if state.active_tab == 8 => {
            let len = state.services.len();
            if len > 0 {
                let current = state.services_table_state.selected().unwrap_or(0);
                state.services_table_state.select(Some((current + 1) % len));
            }
        }
        KeyCode::Up if state.active_tab == 8 => {
            let len = state.services.len();
            if len > 0 {
                let current = state.services_table_state.selected().unwrap_or(0);
                state.services_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
            }
        }
        
        KeyCode::Down if state.active_tab == 9 => {
            let len = state.logs.len();
            if len > 0 {
                let current = state.logs_table_state.selected().unwrap_or(0);
                state.logs_table_state.select(Some((current + 1) % len));
            }
        }
        KeyCode::Up if state.active_tab == 9 => {
            let len = state.logs.len();
            if len > 0 {
                let current = state.logs_table_state.selected().unwrap_or(0);
                state.logs_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
            }
        }
        
        KeyCode::Down if state.active_tab == 10 => {
            let len = state.config_items.len();
            if len > 0 {
                let current = state.config_table_state.selected().unwrap_or(0);
                state.config_table_state.select(Some((current + 1) % len));
            }
        }
        KeyCode::Up if state.active_tab == 10 => {
            let len = state.config_items.len();
            if len > 0 {
                let current = state.config_table_state.selected().unwrap_or(0);
                state.config_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
            }
        }
        
        KeyCode::Char('e') if state.active_tab == 8 => {
            if let Some(idx) = state.services_table_state.selected() {
                if state.has_sudo {
                    state.editing_service = Some(idx);
                    state.edit_buffer.clear();
                }
            }
        }
        
        KeyCode::Char('s') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if service.can_start && state.has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        let _ = sys_mgr.start_service(&service.name);
                        state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Char('x') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if service.can_stop && state.has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        let _ = sys_mgr.stop_service(&service.name);
                        state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Char('r') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if state.has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        let _ = sys_mgr.restart_service(&service.name);
                        state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Char('e') if state.active_tab == 10 => {
            if let Some(idx) = state.config_table_state.selected() {
                if state.has_sudo {
                    state.editing_config = Some(idx);
                    if let Some(item) = state.config_items.get(idx) {
                        state.edit_buffer = item.value.clone();
                    }
                }
            }
        }
        
        KeyCode::Char(c) if state.editing_service.is_some() || state.editing_config.is_some() => {
            state.edit_buffer.push(c);
        }
        
        KeyCode::Backspace if state.editing_service.is_some() || state.editing_config.is_some() => {
            state.edit_buffer.pop();
        }
        
        KeyCode::Enter if state.editing_config.is_some() => {
            if let Some(idx) = state.editing_config {
                let buffer = state.edit_buffer.clone();
                let has_sudo = state.has_sudo;
                if let Some(item) = state.config_items.get_mut(idx) {
                    let key = item.key.clone();
                    item.value = buffer.clone();
                    if has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        match key.as_str() {
                            "hostname" => {
                                let _ = sys_mgr.set_hostname(&buffer);
                            }
                            "timezone" => {
                                let _ = sys_mgr.set_timezone(&buffer);
                            }
                            _ if key.starts_with("GRUB_") => {
                                let _ = sys_mgr.set_grub_config(&key, &buffer);
                            }
                            _ => {}
                        }
                    }
                }
            }
            state.editing_config = None;
            state.edit_buffer.clear();
        }
        

        
        KeyCode::Enter if state.active_tab == 0 => {
            if let Some(selected_index) = state.process_table_state.selected() {
                if let Some(process) = state.dynamic_data.processes.get(selected_index) {
                    if let Ok(pid_val) = process.pid.parse::<usize>() {
                        state.selected_pid = Some(sysinfo::Pid::from(pid_val));
                        state.active_tab = 1;
                    }
                }
            }
        }
        
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
        
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.show_system_processes = !state.show_system_processes;
        }
        
        KeyCode::Char('h') | KeyCode::F(1) => {
        }
        
        _ => {}
    }
    
    Ok(false)
}

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

async fn data_collection_loop(
    app_state: Arc<Mutex<AppState>>,
    data_collector: Arc<Mutex<DataCollector>>,
    config: AppConfig,
) {
    let mut interval = tokio::time::interval(config.get_collection_sleep_duration());
    let mut prev_global_usage = types::GlobalUsage::default();
    
    loop {
        interval.tick().await;
        
        let is_paused = {
            let state = app_state.lock();
            state.paused
        };
        
        if is_paused {
            continue;
        }
        
        let collection_start = Instant::now();
        
        let (selected_pid, show_system_processes, filter_text) = {
            let state = app_state.lock();
            (
                state.selected_pid,
                state.show_system_processes,
                state.filter_text.clone(),
            )
        };
        
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
        
        {
            let mut state = app_state.lock();
            state.dynamic_data = new_data;
            
            if state.process_table_state.selected().is_none() && !state.dynamic_data.processes.is_empty() {
                state.process_table_state.select(Some(0));
            }
        }
        
        let collection_duration = collection_start.elapsed();
        
        if collection_duration > Duration::from_millis(config.refresh_rate_ms / 2) {
            eprintln!("Slow data collection: {:?}", collection_duration);
        }
        
        let remaining_time = config.get_collection_sleep_duration().saturating_sub(collection_duration);
        if remaining_time > Duration::from_millis(10) {
            sleep(remaining_time).await;
        }
    }
}

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

fn check_system_requirements() -> Result<(), AppError> {
    if !atty::is(atty::Stream::Stdout) {
        return Err(AppError::Config(
            "PULS requires a terminal environment".to_string()
        ));
    }
    
    if let Ok((width, height)) = crossterm::terminal::size() {
        if width < 80 || height < 24 {
            eprintln!("Warning: Terminal size {}x{} is smaller than recommended 80x24", width, height);
        }
    }
    
    Ok(())
}

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