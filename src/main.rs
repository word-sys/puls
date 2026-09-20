mod types;
mod utils;
mod config;
mod monitors;
mod ui;
mod language;
mod system_service;
mod error_logger;

use crate::types::{AppState, ProcessSortBy};
use crate::monitors::{DataCollector, SharedDataCollector};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};


use std::sync::Mutex;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Terminal};
use tokio::time::sleep;

use crate::config::{Cli};
use crate::types::AppConfig;
use crate::ui::render_ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    init_logging(cli.verbose)?;
    check_system_requirements()?;
    let config = AppConfig::from(cli);
    
    let telemetry = config.telemetry;
    if telemetry {
        println!("[TELEMETRY] Starting initialization...");
    }
    let startup_start = Instant::now();
    
    let coll_start = Instant::now();
    let data_collector: SharedDataCollector = Arc::new(tokio::sync::Mutex::new(DataCollector::new(config.clone())));
    let coll_duration = coll_start.elapsed();
    if telemetry {
        println!("[TELEMETRY] Collector creation took {:?}", coll_duration);
    }
    
    let sys_info_start = Instant::now();
    let system_info = {
        let collector = data_collector.lock().await;
        collector.get_system_info()
    };
    let sys_info_duration = sys_info_start.elapsed();
    if telemetry {
        println!("[TELEMETRY] System info collection took {:?}", sys_info_duration);
    }
    
    let total_startup_duration = startup_start.elapsed();
    if telemetry {
        println!("[TELEMETRY] Total startup took {:?}", total_startup_duration);
        println!("[TELEMETRY] Entering UI mode in 2 seconds...");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
    
    let app_state = Arc::new(Mutex::new(AppState::default()));
    
    {
        let mut state = app_state.lock().unwrap();
        state.language = config.language;
        state.current_theme = config.theme;
        state.system_info = system_info;
        state.active_tab = config.default_tab.min(12);
        state.process_tree_mode = config.process_tree_view;
        state.temp_unit_fahrenheit = config.temp_unit_fahrenheit;
        state.default_tab = config.default_tab;
        state.refresh_rate_ms = config.refresh_rate_ms;
        
        if config.safe_mode {
            state.system_info.push(("Mode".to_string(), "Safe Mode".to_string()));
        }
        
        let sys_mgr = system_service::SystemManager::new();
        state.has_sudo = sys_mgr.has_sudo_privileges();
    }
    
    let local = tokio::task::LocalSet::new();

    let result = local.run_until(async {
        let app_state_clone = app_state.clone();
        let data_collector_clone = data_collector.clone();
        let config_clone = config.clone();
        tokio::task::spawn_local(async move {
            data_collection_loop(app_state_clone, data_collector_clone, config_clone).await;
        });

        ui_loop(&mut terminal, app_state, data_collector, &config).await
    }).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        log::error!("Application error: {}", e);
        crate::error_logger::log_error(&e.to_string());
    }

    result.map_err(|e| e.into())
}

fn check_and_load_lazy_data(state: &mut AppState) {
    let sys_mgr = system_service::SystemManager::new();
    
    if state.active_tab == 7 && !state.user_sessions_loaded {
        state.user_sessions = sys_mgr.get_logged_in_users();
        state.user_sessions_loaded = true;
    }

    if state.active_tab == 8 && !state.services_loaded {
        state.services = sys_mgr.get_services();
        if !state.services.is_empty() {
            state.services_table_state.select(Some(0));
        }
        state.timers = sys_mgr.get_systemd_timers();
        if !state.timers.is_empty() {
            state.timers_table_state.select(Some(0));
        }
        state.services_loaded = true;
        state.timers_loaded = true;
    }
    
    if state.active_tab == 9 && !state.logs_loaded {
        state.boots = sys_mgr.get_boots();
        if !state.boots.is_empty() {
            state.current_boot_idx = 0;
        }
        state.boots_loaded = true;
        
        let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
        state.logs = sys_mgr.get_logs(1000, None, boot_id);
        if !state.logs.is_empty() {
            state.logs_table_state.select(Some(0));
        }
        state.logs_loaded = true;
    }
    
    if state.active_tab == 10 && !state.config_loaded {
        state.config_items = sys_mgr.get_grub_config();
        if !state.config_items.is_empty() {
            state.config_table_state.select(Some(0));
        }
        state.config_loaded = true;
    }
}

async fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: Arc<Mutex<AppState>>,
    data_collector: SharedDataCollector,
    config: &AppConfig,
) -> io::Result<()> {
    let mut last_render = Instant::now();
    
    loop {
        let now = Instant::now();
        
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(key) => {
                    let should_quit = handle_key_event(key, &app_state, &data_collector, config)?;
                    if should_quit {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(mouse, &app_state, config)?;
                }
                _ => {}
            }
        }
        
        let ui_refresh_interval = {
            let state = app_state.lock().unwrap();
            Duration::from_millis(state.refresh_rate_ms.max(100))
        };
        
        if now.duration_since(last_render) >= ui_refresh_interval {
            {
                let mut state = app_state.lock().unwrap();
                check_and_load_lazy_data(&mut state);
                let translator = crate::language::Translator::new(state.language);
                terminal.draw(|f| render_ui(f, &mut state, config.safe_mode, &translator))?;
            }
            last_render = now;
        }
        
        sleep(Duration::from_millis(2)).await;
    }
}

fn handle_key_event(
    key: crossterm::event::KeyEvent,
    app_state: &Arc<Mutex<AppState>>,
    data_collector: &SharedDataCollector,
    _config: &AppConfig,
) -> io::Result<bool> {
    let mut state = app_state.lock().unwrap();
    
    if state.show_settings_modal {
        match key.code {
            KeyCode::Esc | KeyCode::F(2) | KeyCode::Char('q') | KeyCode::Char('Q') => {
                state.show_settings_modal = false;
                persist_settings(&state);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.settings_selected_idx = state.settings_selected_idx.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.settings_selected_idx < 5 {
                    state.settings_selected_idx += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                modify_setting(&mut state, -1);
                persist_settings(&state);
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char(' ') => {
                modify_setting(&mut state, 1);
                persist_settings(&state);
            }
            _ => {}
        }
        return Ok(false);
    }
    
    let is_editing = state.editing_config.is_some() || state.editing_service.is_some() || state.editing_filter;
    if is_editing {
        match key.code {
            KeyCode::Esc => {
                if state.editing_filter {
                    state.editing_filter = false;
                    state.edit_buffer.clear();
                } else {
                    state.editing_config = None;
                    state.editing_service = None;
                    state.edit_buffer.clear();
                }
            }
            KeyCode::Enter => {
                if state.editing_filter {
                    if state.active_tab == 9 {
                        state.log_filter = state.edit_buffer.clone();
                        state.editing_filter = false;
                        state.edit_buffer.clear();
                        let sys_mgr = system_service::SystemManager::new();
                        let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
                        state.logs = sys_mgr.get_logs(1000, Some(&state.log_filter), boot_id);
                        state.logs_table_state.select(Some(0));
                    } else if state.active_tab == 1 {
                        state.filter_text = state.edit_buffer.clone();
                        state.editing_filter = false;
                        state.edit_buffer.clear();
                        state.process_table_state.select(Some(0));
                    }
                } else if let Some(idx) = state.editing_config {
                    let new_val = state.edit_buffer.clone();
                    if let Some(item) = state.config_items.get_mut(idx) {
                        item.value = new_val;
                    }
                    state.editing_config = None;
                    state.edit_buffer.clear();
                } else if state.editing_service.is_some() {
                    state.editing_service = None;
                    state.edit_buffer.clear();
                }
            }
            KeyCode::Backspace => {
                state.edit_buffer.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT) => {
                state.edit_buffer.push(c);
            }
            _ => {}
        }
        return Ok(false);
    }
    
    if let Some((pid, _name, ref mut sel_idx)) = state.signal_modal.as_mut() {
        let pid = *pid;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if *sel_idx > 0 {
                    *sel_idx -= 1;
                } else {
                    *sel_idx = crate::types::POSIX_SIGNALS.len() - 1;
                }
                return Ok(false);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *sel_idx < crate::types::POSIX_SIGNALS.len() - 1 {
                    *sel_idx += 1;
                } else {
                    *sel_idx = 0;
                }
                return Ok(false);
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') | KeyCode::Char('N') => {
                state.signal_modal = None;
                return Ok(false);
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                let sig_num = crate::types::POSIX_SIGNALS[*sel_idx].num;
                state.signal_modal = None;

                let output = std::process::Command::new("kill")
                    .args([&format!("-{}", sig_num), &pid.to_string()])
                    .output();

                match output {
                    Ok(out) if !out.status.success() => {
                        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        state.service_status_modal = Some(("Signal Failed".to_string(), err));
                    }
                    Err(e) => {
                        state.service_status_modal = Some(("Signal Failed".to_string(), e.to_string()));
                    }
                    _ => {}
                }
                return Ok(false);
            }
            _ => {
                return Ok(false);
            }
        }
    }

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') if state.pending_grub_update_confirmation => {
            let sys_mgr = system_service::SystemManager::new();
            let changes: Vec<(String, String)> = state.config_items.iter()
                .filter(|item| item.value != item.original_value)
                .map(|item| (item.key.clone(), item.value.clone()))
                .collect();

            match sys_mgr.apply_config_changes(&changes) {
                Ok(msg) => {
                    for item in &mut state.config_items {
                        item.original_value = item.value.clone();
                    }
                    state.service_status_modal = Some(("Success".to_string(), msg));
                }
                Err(e) => {
                    state.service_status_modal = Some(("Error".to_string(), e));
                }
            }
            state.pending_grub_update_confirmation = false;
        }

        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc if state.pending_grub_update_confirmation => {
            state.pending_grub_update_confirmation = false;
        }

        _ if state.pending_grub_update_confirmation => {
            return Ok(false);
        }

        KeyCode::Esc | KeyCode::Enter if state.viewing_log.is_some() => {
            state.viewing_log = None;
        }
        
        _ if state.viewing_log.is_some() => {
            return Ok(false);
        }
        
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            if state.show_settings_modal {
                state.show_settings_modal = false;
                persist_settings(&state);
                return Ok(false);
            }
            if state.signal_modal.is_some() {
                state.signal_modal = None;
                return Ok(false);
            }
            if state.pending_kill_pid.is_some() {
                state.pending_kill_pid = None;
                return Ok(false);
            }
            if state.pending_config_confirmation.is_some() {
                state.pending_config_confirmation = None;
                return Ok(false);
            }
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
            if state.pending_container_action.is_some() {
                state.pending_container_action = None;
                return Ok(false);
            }
            if state.viewing_container_logs.is_some() {
                state.viewing_container_logs = None;
                return Ok(false);
            }
            if state.active_tab == 1 && state.selected_pid.is_some() {
                state.selected_pid = None;
                state.process_detail_subtab = 0;
                state.process_detail_scroll = 0;
                return Ok(false);
            }
            if state.active_tab == 1 && !state.filter_text.is_empty() {
                state.filter_text.clear();
                state.process_table_state.select(Some(0));
                return Ok(false);
            }
            return Ok(true);
        }

        KeyCode::Up if state.viewing_container_logs.is_some() => {
            if let Some((_, _, _, ref mut scroll)) = state.viewing_container_logs {
                *scroll = scroll.saturating_sub(1);
            }
        }
        KeyCode::Down if state.viewing_container_logs.is_some() => {
            if let Some((_, _, ref logs, ref mut scroll)) = state.viewing_container_logs {
                let max_scroll = logs.len().saturating_sub(1);
                *scroll = (*scroll + 1).min(max_scroll);
            }
        }
        KeyCode::PageUp if state.viewing_container_logs.is_some() => {
            if let Some((_, _, _, ref mut scroll)) = state.viewing_container_logs {
                *scroll = scroll.saturating_sub(15);
            }
        }
        KeyCode::PageDown if state.viewing_container_logs.is_some() => {
            if let Some((_, _, ref logs, ref mut scroll)) = state.viewing_container_logs {
                let max_scroll = logs.len().saturating_sub(1);
                *scroll = (*scroll + 15).min(max_scroll);
            }
        }
        KeyCode::Home if state.viewing_container_logs.is_some() => {
            if let Some((_, _, _, ref mut scroll)) = state.viewing_container_logs {
                *scroll = 0;
            }
        }
        KeyCode::End if state.viewing_container_logs.is_some() => {
            if let Some((_, _, ref logs, ref mut scroll)) = state.viewing_container_logs {
                *scroll = logs.len().saturating_sub(1);
            }
        }

        KeyCode::Enter | KeyCode::Char('l') if state.active_tab == 11 && state.pending_container_action.is_none() && state.viewing_container_logs.is_none() => {
             if let Some(idx) = state.container_table_state.selected() {
                 if let Some(container) = state.dynamic_data.containers.get(idx) {
                     let container_id = container.id.clone();
                     let container_name = container.name.clone();
                     
                     let app_state_clone = app_state.clone();
                     let data_collector_clone = data_collector.clone();

                     state.viewing_container_logs = Some((
                         container_name.clone(),
                         container_id.clone(),
                         vec!["Fetching container logs...".to_string()],
                         0,
                     ));
                     
                     tokio::task::spawn_local(async move {
                         let logs_res = {
                             let dc = data_collector_clone.lock().await;
                             dc.get_container_logs(&container_id).await
                         };
                         let mut st = app_state_clone.lock().unwrap();
                         if let Some((_, ref id, ref mut logs, _)) = st.viewing_container_logs {
                             if *id == container_id {
                                 match logs_res {
                                     Ok(l) => {
                                         if l.is_empty() {
                                             *logs = vec!["(No log entries found for container)".to_string()];
                                         } else {
                                             *logs = l;
                                         }
                                     }
                                     Err(e) => {
                                         *logs = vec![format!("Failed to retrieve logs: {}", e)];
                                     }
                                 }
                             }
                         }
                     });
                 }
             }
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
        
        KeyCode::Char('g') if state.active_tab == 8 && state.service_status_modal.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    let sys_mgr = system_service::SystemManager::new();
                    let logs = sys_mgr.get_service_logs(&service.name);
                    state.service_status_modal = Some((format!("Logs: {}", service.name), logs));
                }
            }
        }

        KeyCode::Char('/') if state.active_tab == 9 && !state.editing_filter => {
             state.editing_filter = true;
             state.edit_buffer = state.log_filter.clone();
        }

        KeyCode::Char('/') if state.active_tab == 1 && !state.editing_filter => {
             state.editing_filter = true;
             state.edit_buffer = state.filter_text.clone();
        }

        // Filter and edit keys are now handled in the early is_editing check at the top of handle_key_event

        KeyCode::Char('>') | KeyCode::Right if state.active_tab == 9 && !state.editing_filter => {
            if !state.boots.is_empty() && state.current_boot_idx > 0 {
                state.current_boot_idx -= 1;
                let sys_mgr = system_service::SystemManager::new();
                let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
                let filter = if state.log_filter.is_empty() { None } else { Some(state.log_filter.as_str()) };
                state.logs = sys_mgr.get_logs(1000, filter, boot_id);
                state.logs_table_state.select(Some(0));
            }
        }

        KeyCode::Char('<') | KeyCode::Left if state.active_tab == 9 && !state.editing_filter => {
            if !state.boots.is_empty() && state.current_boot_idx < state.boots.len() - 1 {
                state.current_boot_idx += 1;
                let sys_mgr = system_service::SystemManager::new();
                let boot_id = state.boots.get(state.current_boot_idx).map(|b| b.id.as_str());
                let filter = if state.log_filter.is_empty() { None } else { Some(state.log_filter.as_str()) };
                state.logs = sys_mgr.get_logs(1000, filter, boot_id);
                state.logs_table_state.select(Some(0));
            }
        }

        KeyCode::Char('p') | KeyCode::Char('P') if state.active_tab != 11 => {
            state.paused = !state.paused;
        }
        
        KeyCode::Tab if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = (state.process_detail_subtab + 1) % 4;
            state.process_detail_scroll = 0;
        }
        KeyCode::BackTab if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = (state.process_detail_subtab + 3) % 4;
            state.process_detail_scroll = 0;
        }
        KeyCode::Right if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = (state.process_detail_subtab + 1) % 4;
            state.process_detail_scroll = 0;
        }
        KeyCode::Left if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = (state.process_detail_subtab + 3) % 4;
            state.process_detail_scroll = 0;
        }
        KeyCode::Char('1') if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = 0;
            state.process_detail_scroll = 0;
        }
        KeyCode::Char('2') if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = 1;
            state.process_detail_scroll = 0;
        }
        KeyCode::Char('3') if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = 2;
            state.process_detail_scroll = 0;
        }
        KeyCode::Char('4') if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_subtab = 3;
            state.process_detail_scroll = 0;
        }
        KeyCode::Down if state.active_tab == 1 && state.selected_pid.is_some() => {
            let max_items = state.dynamic_data.detailed_process.as_ref().map_or(0, |proc| {
                match state.process_detail_subtab {
                    1 => proc.fds.len(),
                    2 => proc.thread_list.len(),
                    3 => proc.environ.len(),
                    _ => 0,
                }
            });
            if max_items > 0 {
                let max_scroll = max_items.saturating_sub(1);
                state.process_detail_scroll = (state.process_detail_scroll + 1).min(max_scroll);
            }
        }
        KeyCode::Up if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_scroll = state.process_detail_scroll.saturating_sub(1);
        }
        KeyCode::PageDown if state.active_tab == 1 && state.selected_pid.is_some() => {
            let max_items = state.dynamic_data.detailed_process.as_ref().map_or(0, |proc| {
                match state.process_detail_subtab {
                    1 => proc.fds.len(),
                    2 => proc.thread_list.len(),
                    3 => proc.environ.len(),
                    _ => 0,
                }
            });
            if max_items > 0 {
                let max_scroll = max_items.saturating_sub(1);
                state.process_detail_scroll = (state.process_detail_scroll + 15).min(max_scroll);
            }
        }
        KeyCode::PageUp if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_scroll = state.process_detail_scroll.saturating_sub(15);
        }
        KeyCode::Home if state.active_tab == 1 && state.selected_pid.is_some() => {
            state.process_detail_scroll = 0;
        }
        KeyCode::End if state.active_tab == 1 && state.selected_pid.is_some() => {
            let max_items = state.dynamic_data.detailed_process.as_ref().map_or(0, |proc| {
                match state.process_detail_subtab {
                    1 => proc.fds.len(),
                    2 => proc.thread_list.len(),
                    3 => proc.environ.len(),
                    _ => 0,
                }
            });
            state.process_detail_scroll = max_items.saturating_sub(1);
        }

        KeyCode::Down if state.active_tab == 5 => {
            let total = state.dynamic_data.sockets.len();
            if total > 0 {
                state.network_socket_scroll = (state.network_socket_scroll + 1).min(total.saturating_sub(1));
            }
        }
        KeyCode::Up if state.active_tab == 5 => {
            state.network_socket_scroll = state.network_socket_scroll.saturating_sub(1);
        }
        KeyCode::PageDown if state.active_tab == 5 => {
            let total = state.dynamic_data.sockets.len();
            if total > 0 {
                state.network_socket_scroll = (state.network_socket_scroll + 15).min(total.saturating_sub(1));
            }
        }
        KeyCode::PageUp if state.active_tab == 5 => {
            state.network_socket_scroll = state.network_socket_scroll.saturating_sub(15);
        }
        KeyCode::Home if state.active_tab == 5 => {
            state.network_socket_scroll = 0;
        }
        KeyCode::End if state.active_tab == 5 => {
            let total = state.dynamic_data.sockets.len();
            state.network_socket_scroll = total.saturating_sub(1);
        }

        KeyCode::Tab => {
            state.active_tab = (state.active_tab + 1) % 13;
            state.selected_pid = None;
            state.network_socket_scroll = 0;
            state.cpu_cores_scroll = 0;
            state.pending_container_action = None;
            state.viewing_container_logs = None;
        }
        KeyCode::BackTab => {
            state.active_tab = (state.active_tab + 12) % 13;
            state.selected_pid = None;
            state.network_socket_scroll = 0;
            state.cpu_cores_scroll = 0;
            state.pending_container_action = None;
            state.viewing_container_logs = None;
        }
        
        KeyCode::Char('1') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 0; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('2') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 1; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('3') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 2; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('4') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 3; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('5') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 4; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('6') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 5; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('7') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 6; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('8') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 7; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('9') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 8; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('0') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 9; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('-') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 10; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('=') if state.editing_config.is_none() && state.editing_service.is_none() => { state.active_tab = 11; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        KeyCode::Char('+') if state.editing_config.is_none() && state.editing_service.is_none() && state.active_tab != 8 => { state.active_tab = 12; state.selected_pid = None; state.network_socket_scroll = 0; state.cpu_cores_scroll = 0; state.pending_container_action = None; state.viewing_container_logs = None; },
        
        KeyCode::F(2) | KeyCode::Char('S') => {
            state.show_settings_modal = true;
            return Ok(false);
        }
        KeyCode::Char('s') if state.active_tab != 8 && state.active_tab != 11 => {
            state.show_settings_modal = true;
            return Ok(false);
        }

        KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::F(5) if state.active_tab == 1 && state.selected_pid.is_none() => {
            state.process_tree_mode = !state.process_tree_mode;
            let sort_by = state.sort_by.clone();
            let sort_asc = state.sort_ascending;
            if state.process_tree_mode {
                crate::monitors::system_monitor::build_process_tree(
                    &mut state.dynamic_data.processes,
                    &sort_by,
                    sort_asc,
                    1,
                );
            } else {
                crate::monitors::system_monitor::sort_processes(
                    &mut state.dynamic_data.processes,
                    &sort_by,
                    sort_asc,
                    1,
                );
            }
            persist_settings(&state);
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            state.current_theme = (state.current_theme + 1) % crate::ui::colors::THEME_COUNT;
            persist_settings(&state);
        }
        KeyCode::Char('L') => {
            state.language = state.language.next();
            persist_settings(&state);
        }
        KeyCode::Char('l') if state.active_tab != 8 && state.active_tab != 11 => {
            state.language = state.language.next();
            persist_settings(&state);
        }
        
        KeyCode::Down if state.active_tab == 1 && state.selected_pid.is_none() => {
            handle_process_navigation(&mut state, true);
        }
        KeyCode::Up if state.active_tab == 1 && state.selected_pid.is_none() => {
            handle_process_navigation(&mut state, false);
        }
        
        KeyCode::Down if state.active_tab == 2 => {
            state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_add(1);
        }
        KeyCode::Up if state.active_tab == 2 => {
            state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_sub(1);
        }
        KeyCode::PageDown if state.active_tab == 2 => {
            state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_add(6);
        }
        KeyCode::PageUp if state.active_tab == 2 => {
            state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_sub(6);
        }
        KeyCode::Home if state.active_tab == 2 => {
            state.cpu_cores_scroll = 0;
        }
        KeyCode::End if state.active_tab == 2 => {
            state.cpu_cores_scroll = 999;
        }
        
        KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::F(9) if state.active_tab == 1 && state.selected_pid.is_none() && state.signal_modal.is_none() => {
            if let Some(idx) = state.process_table_state.selected() {
                if idx < state.dynamic_data.processes.len() {
                    let proc_info = &state.dynamic_data.processes[idx];
                    if let Ok(pid_num) = proc_info.pid.parse::<usize>() {
                         let pid = sysinfo::Pid::from(pid_num);
                         state.signal_modal = Some((pid, proc_info.name.clone(), 0));
                    }
                }
            }
        }

        KeyCode::Char('[') | KeyCode::Char(']') if state.active_tab == 1 && state.selected_pid.is_none() && state.signal_modal.is_none() => {
            if let Some(idx) = state.process_table_state.selected() {
                if let Some(proc_info) = state.dynamic_data.processes.get(idx) {
                    let pid_str = proc_info.pid.clone();
                    let current_nice = proc_info.nice;
                    let delta = if key.code == KeyCode::Char('[') { -1 } else { 1 };
                    let new_nice = (current_nice + delta).clamp(-20, 19);

                    let output = std::process::Command::new("renice")
                        .args([&new_nice.to_string(), "-p", &pid_str])
                        .output();

                    match output {
                        Ok(out) if out.status.success() => {
                            if let Some(proc_mut) = state.dynamic_data.processes.get_mut(idx) {
                                proc_mut.nice = new_nice;
                            }
                        }
                        Ok(out) => {
                            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            state.service_status_modal = Some(("Renice Failed".to_string(), err));
                        }
                        Err(e) => {
                            state.service_status_modal = Some(("Renice Failed".to_string(), e.to_string()));
                        }
                    }
                }
            }
        }
        
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter if state.pending_kill_pid.is_some() => {
            if let Some(pid) = state.pending_kill_pid.take() {
                use std::process::Command;
                let output = Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .output();
                
                match output {
                    Ok(out) if !out.status.success() => {
                        let err = String::from_utf8_lossy(&out.stderr).to_string();
                        state.service_status_modal = Some(("Kill Failed".to_string(), err));
                    }
                    Err(e) => {
                        state.service_status_modal = Some(("Kill Failed".to_string(), e.to_string()));
                    }
                    _ => {}
                }
                
                state.selected_pid = None;
            }
        }
        
        KeyCode::Char('n') | KeyCode::Char('N') if state.pending_kill_pid.is_some() => {
            state.pending_kill_pid = None;
        }

        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter if state.pending_service_action.is_some() => {
             if let Some((action, service_name)) = state.pending_service_action.take() {
                let sys_mgr = system_service::SystemManager::new();
                let result = match action.as_str() {
                    "stop" => sys_mgr.stop_service(&service_name),
                    _ => Ok(()),
                };

                match result {
                    Ok(_) => state.service_status_modal = Some(("Success".to_string(), format!("Stopped {}", service_name))),
                    Err(e) => state.service_status_modal = Some(("Error".to_string(), e)),
                }
                state.services = sys_mgr.get_services();
             }
        }

        KeyCode::Char('n') | KeyCode::Char('N') if state.pending_service_action.is_some() => {
             state.pending_service_action = None;
        }

        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter if state.pending_container_action.is_some() => {
            if let Some((action, name, id)) = state.pending_container_action.take() {
                let app_state_clone = app_state.clone();
                let data_collector_clone = data_collector.clone();
                tokio::task::spawn_local(async move {
                    let result = {
                        let dc = data_collector_clone.lock().await;
                        match action.as_str() {
                            "start" => dc.start_container(&id).await,
                            "stop" => dc.stop_container(&id).await,
                            "restart" => dc.restart_container(&id).await,
                            "pause" => dc.pause_container(&id).await,
                            "unpause" => dc.unpause_container(&id).await,
                            _ => Ok(()),
                        }
                    };
                    let mut st = app_state_clone.lock().unwrap();
                    match result {
                        Ok(_) => {
                            st.service_status_modal = Some((
                                "Success".to_string(),
                                format!("Container '{}' successfully {}.", name, match action.as_str() {
                                    "start" => "started",
                                    "stop" => "stopped",
                                    "restart" => "restarted",
                                    "pause" => "paused",
                                    "unpause" => "unpaused",
                                    _ => "processed",
                                }),
                            ));
                        }
                        Err(e) => {
                            st.service_status_modal = Some(("Container Action Error".to_string(), e));
                        }
                    }
                });
            }
        }

        KeyCode::Char('n') | KeyCode::Char('N') if state.pending_container_action.is_some() => {
            state.pending_container_action = None;
        }
        
        KeyCode::Left | KeyCode::Right if state.active_tab == 8 && state.pending_service_action.is_none() => {
            state.services_subtab = (state.services_subtab + 1) % 2;
        }

        KeyCode::Down if state.active_tab == 8 && state.pending_service_action.is_none() => {
            if state.services_subtab == 0 {
                let len = state.services.len();
                if len > 0 {
                    let current = state.services_table_state.selected().unwrap_or(0);
                    state.services_table_state.select(Some((current + 1) % len));
                }
            } else {
                let len = state.timers.len();
                if len > 0 {
                    let current = state.timers_table_state.selected().unwrap_or(0);
                    state.timers_table_state.select(Some((current + 1) % len));
                }
            }
        }
        KeyCode::Up if state.active_tab == 8 && state.pending_service_action.is_none() => {
            if state.services_subtab == 0 {
                let len = state.services.len();
                if len > 0 {
                    let current = state.services_table_state.selected().unwrap_or(0);
                    state.services_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
                }
            } else {
                let len = state.timers.len();
                if len > 0 {
                    let current = state.timers_table_state.selected().unwrap_or(0);
                    state.timers_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
                }
            }
        }
        KeyCode::PageDown if state.active_tab == 8 && state.pending_service_action.is_none() => {
            if state.services_subtab == 0 {
                let len = state.services.len();
                if len > 0 {
                    let current = state.services_table_state.selected().unwrap_or(0);
                    state.services_table_state.select(Some((current + 10).min(len - 1)));
                }
            } else {
                let len = state.timers.len();
                if len > 0 {
                    let current = state.timers_table_state.selected().unwrap_or(0);
                    state.timers_table_state.select(Some((current + 10).min(len - 1)));
                }
            }
        }
        KeyCode::PageUp if state.active_tab == 8 && state.pending_service_action.is_none() => {
            if state.services_subtab == 0 {
                let len = state.services.len();
                if len > 0 {
                    let current = state.services_table_state.selected().unwrap_or(0);
                    state.services_table_state.select(Some(current.saturating_sub(10)));
                }
            } else {
                let len = state.timers.len();
                if len > 0 {
                    let current = state.timers_table_state.selected().unwrap_or(0);
                    state.timers_table_state.select(Some(current.saturating_sub(10)));
                }
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
        
        KeyCode::Enter if state.active_tab == 9 => {
            if let Some(idx) = state.logs_table_state.selected() {
                if let Some(log) = state.logs.get(idx) {
                    state.viewing_log = Some(log.clone());
                }
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

        KeyCode::Down if state.active_tab == 11 => {
            let len = state.dynamic_data.containers.len();
            if len > 0 {
                let current = state.container_table_state.selected().unwrap_or(0);
                state.container_table_state.select(Some((current + 1) % len));
            }
        }
        KeyCode::Up if state.active_tab == 11 => {
            let len = state.dynamic_data.containers.len();
            if len > 0 {
                let current = state.container_table_state.selected().unwrap_or(0);
                state.container_table_state.select(Some(if current == 0 { len - 1 } else { current - 1 }));
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
        
        KeyCode::Char('s') if state.active_tab == 8 && state.editing_service.is_none() && state.pending_service_action.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if service.can_start && state.has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        let service_name = service.name.clone();
                        match sys_mgr.start_service(&service_name) {
                            Ok(_) => state.service_status_modal = Some(("Success".to_string(), format!("Started {}", service_name))),
                            Err(e) => state.service_status_modal = Some(("Error".to_string(), e)),
                        }
                        state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Char('x') if state.active_tab == 8 && state.editing_service.is_none() && state.pending_service_action.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if service.can_stop && state.has_sudo {
                        state.pending_service_action = Some(("stop".to_string(), service.name.clone()));
                    }
                }
            }
        }
        
        KeyCode::Char('r') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if state.has_sudo {
                        let sys_mgr = system_service::SystemManager::new();
                        let service_name = service.name.clone();
                        match sys_mgr.restart_service(&service_name) {
                            Ok(_) => state.service_status_modal = Some(("Success".to_string(), format!("Restarted {}", service_name))),
                            Err(e) => state.service_status_modal = Some(("Error".to_string(), e)),
                        }
                        state.services = sys_mgr.get_services();
                    }
                }
            }
        }

        KeyCode::Char('s') if state.active_tab == 11 && state.pending_container_action.is_none() && state.viewing_container_logs.is_none() => {
            if let Some(idx) = state.container_table_state.selected() {
                if let Some(c) = state.dynamic_data.containers.get(idx) {
                    state.pending_container_action = Some(("start".to_string(), c.name.clone(), c.id.clone()));
                }
            }
        }

        KeyCode::Char('x') if state.active_tab == 11 && state.pending_container_action.is_none() && state.viewing_container_logs.is_none() => {
            if let Some(idx) = state.container_table_state.selected() {
                if let Some(c) = state.dynamic_data.containers.get(idx) {
                    state.pending_container_action = Some(("stop".to_string(), c.name.clone(), c.id.clone()));
                }
            }
        }

        KeyCode::Char('r') if state.active_tab == 11 && state.pending_container_action.is_none() && state.viewing_container_logs.is_none() => {
            if let Some(idx) = state.container_table_state.selected() {
                if let Some(c) = state.dynamic_data.containers.get(idx) {
                    state.pending_container_action = Some(("restart".to_string(), c.name.clone(), c.id.clone()));
                }
            }
        }

        KeyCode::Char('p') | KeyCode::Char('P') if state.active_tab == 11 && state.pending_container_action.is_none() && state.viewing_container_logs.is_none() => {
            if let Some(idx) = state.container_table_state.selected() {
                if let Some(c) = state.dynamic_data.containers.get(idx) {
                    let action = if c.status.to_lowercase().contains("pause") {
                        "unpause".to_string()
                    } else {
                        "pause".to_string()
                    };
                    state.pending_container_action = Some((action, c.name.clone(), c.id.clone()));
                }
            }
        }

        KeyCode::Char('+') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if state.has_sudo {
                         let sys_mgr = system_service::SystemManager::new();
                         let service_name = service.name.clone();
                         match sys_mgr.enable_service(&service_name) {
                             Ok(_) => state.service_status_modal = Some(("Success".to_string(), format!("Enabled {}", service_name))),
                             Err(e) => state.service_status_modal = Some(("Error".to_string(), e)),
                         }
                         state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Char('_') if state.active_tab == 8 && state.editing_service.is_none() => {
            if let Some(idx) = state.services_table_state.selected() {
                if let Some(service) = state.services.get(idx) {
                    if state.has_sudo {
                         let sys_mgr = system_service::SystemManager::new();
                         let service_name = service.name.clone();
                         match sys_mgr.disable_service(&service_name) {
                             Ok(_) => state.service_status_modal = Some(("Success".to_string(), format!("Disabled {}", service_name))),
                             Err(e) => state.service_status_modal = Some(("Error".to_string(), e)),
                         }
                         state.services = sys_mgr.get_services();
                    }
                }
            }
        }
        
        KeyCode::Enter if state.active_tab == 10 && state.editing_config.is_none() => {
            if let Some(idx) = state.config_table_state.selected() {
                if state.has_sudo {
                    state.editing_config = Some(idx);
                    if let Some(item) = state.config_items.get(idx) {
                        state.edit_buffer = item.value.clone();
                    }
                }
            }
        }
        
        KeyCode::Char('u') | KeyCode::Char('U') if state.active_tab == 10 && state.editing_config.is_none() && !state.pending_grub_update_confirmation => {
            let has_changes = state.config_items.iter().any(|item| item.value != item.original_value);
            if has_changes {
                state.pending_grub_update_confirmation = true;
            }
        }

        // Config and service edit keys are now handled in the early is_editing check at the top of handle_key_event
        

        
        KeyCode::Enter if state.active_tab == 1 && state.selected_pid.is_none() => {
            if let Some(selected_index) = state.process_table_state.selected() {
                if let Some(process) = state.dynamic_data.processes.get(selected_index) {
                    if let Ok(pid_val) = process.pid.parse::<usize>() {
                        state.selected_pid = Some(sysinfo::Pid::from(pid_val));
                        state.process_detail_subtab = 0;
                        state.process_detail_scroll = 0;
                    }
                }
            }
        }
        
        KeyCode::Char('c') if state.active_tab == 1 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Cpu;
            state.sort_ascending = !state.sort_ascending;
        }
        KeyCode::Char('m') if state.active_tab == 1 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Memory;
            state.sort_ascending = !state.sort_ascending;
        }
        KeyCode::Char('n') if state.active_tab == 1 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::Name;
            state.sort_ascending = !state.sort_ascending;
        }
        KeyCode::Char('g') if state.active_tab == 1 && key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.sort_by = ProcessSortBy::General;
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

fn persist_settings(state: &AppState) {
    let _ = crate::config::save_user_settings(
        state.language,
        state.current_theme,
        state.refresh_rate_ms,
        state.temp_unit_fahrenheit,
        state.default_tab,
        state.process_tree_mode,
    );
}

fn modify_setting(state: &mut AppState, direction: i32) {
    match state.settings_selected_idx {
        0 => {
            let rates = [500u64, 1000, 2000, 5000];
            let cur_idx = rates.iter().position(|&r| r == state.refresh_rate_ms).unwrap_or(1);
            let next_idx = if direction >= 0 {
                (cur_idx + 1) % rates.len()
            } else {
                (cur_idx + rates.len() - 1) % rates.len()
            };
            state.refresh_rate_ms = rates[next_idx];
        }
        1 => {
            state.temp_unit_fahrenheit = !state.temp_unit_fahrenheit;
        }
        2 => {
            let count = crate::ui::colors::THEME_COUNT;
            state.current_theme = if direction >= 0 {
                (state.current_theme + 1) % count
            } else {
                (state.current_theme + count - 1) % count
            };
        }
        3 => {
            let count = 13;
            state.default_tab = if direction >= 0 {
                (state.default_tab + 1) % count
            } else {
                (state.default_tab + count - 1) % count
            };
        }
        4 => {
            state.process_tree_mode = !state.process_tree_mode;
            if state.active_tab == 1 {
                let sort_by = state.sort_by.clone();
                let sort_asc = state.sort_ascending;
                if state.process_tree_mode {
                    crate::monitors::system_monitor::build_process_tree(
                        &mut state.dynamic_data.processes,
                        &sort_by,
                        sort_asc,
                        1,
                    );
                } else {
                    crate::monitors::system_monitor::sort_processes(
                        &mut state.dynamic_data.processes,
                        &sort_by,
                        sort_asc,
                        1,
                    );
                }
            }
        }
        5 => {
            state.language = if direction >= 0 {
                state.language.next()
            } else {
                state.language.prev()
            };
        }
        _ => {}
    }
}

fn handle_mouse_event(
    mouse: MouseEvent,
    app_state: &Arc<Mutex<AppState>>,
    _config: &AppConfig,
) -> io::Result<()> {
    let mut state = app_state.lock().unwrap();

    // 1. Settings modal active
    if state.show_settings_modal {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if state.settings_selected_idx < 5 {
                    state.settings_selected_idx += 1;
                }
            }
            MouseEventKind::ScrollUp => {
                state.settings_selected_idx = state.settings_selected_idx.saturating_sub(1);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 24));
                let width = 68.min(term_width.saturating_sub(4));
                let height = 20.min(term_height.saturating_sub(2));
                let popup_x = (term_width.saturating_sub(width)) / 2;
                let popup_y = (term_height.saturating_sub(height)) / 2;

                if mouse.column < popup_x || mouse.column >= popup_x + width
                    || mouse.row < popup_y || mouse.row >= popup_y + height
                {
                    state.show_settings_modal = false;
                    persist_settings(&state);
                } else {
                    for idx in 0..6 {
                        let item_row = popup_y + 3 + (idx as u16 * 2);
                        if mouse.row == item_row {
                            if state.settings_selected_idx == idx {
                                modify_setting(&mut state, 1);
                            } else {
                                state.settings_selected_idx = idx;
                            }
                            persist_settings(&state);
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // 2. Normal mode mouse handling
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let (term_width, _) = crossterm::terminal::size().unwrap_or((80, 24));
            
            // Tab bar is rows 0, 1, 2
            if mouse.row <= 2 {
                let translator = crate::language::Translator::new(state.language);
                if let Some(tab_idx) = crate::ui::get_tab_at_column(mouse.column, &translator) {
                    state.active_tab = tab_idx;
                    state.selected_pid = None;
                    state.network_socket_scroll = 0;
                    state.cpu_cores_scroll = 0;
                    state.pending_container_action = None;
                    state.viewing_container_logs = None;
                } else if mouse.column >= term_width.saturating_sub(30) {
                    state.show_settings_modal = true;
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if let Some((_, _, ref mut sel_idx)) = state.signal_modal {
                if *sel_idx < crate::types::POSIX_SIGNALS.len() - 1 {
                    *sel_idx += 1;
                } else {
                    *sel_idx = 0;
                }
            } else if let Some((_, _, ref logs, ref mut scroll)) = state.viewing_container_logs {
                let max_scroll = logs.len().saturating_sub(1);
                *scroll = (*scroll + 3).min(max_scroll);
            } else {
                match state.active_tab {
                    1 => {
                        if state.selected_pid.is_some() {
                            state.process_detail_scroll = state.process_detail_scroll.saturating_add(3);
                        } else {
                            handle_process_navigation(&mut state, true);
                        }
                    }
                    2 => {
                        state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_add(1);
                    }
                    5 => {
                        let max_s = state.dynamic_data.sockets.len().saturating_sub(1);
                        state.network_socket_scroll = (state.network_socket_scroll + 1).min(max_s);
                    }
                    8 => {
                        if state.services_subtab == 0 {
                            let cur = state.services_table_state.selected().unwrap_or(0);
                            if cur + 1 < state.services.len() {
                                state.services_table_state.select(Some(cur + 1));
                            }
                        } else {
                            let cur = state.timers_table_state.selected().unwrap_or(0);
                            if cur + 1 < state.timers.len() {
                                state.timers_table_state.select(Some(cur + 1));
                            }
                        }
                    }
                    9 => {
                        let cur = state.logs_table_state.selected().unwrap_or(0);
                        if cur + 1 < state.logs.len() {
                            state.logs_table_state.select(Some(cur + 1));
                        }
                    }
                    10 => {
                        let cur = state.config_table_state.selected().unwrap_or(0);
                        if cur + 1 < state.config_items.len() {
                            state.config_table_state.select(Some(cur + 1));
                        }
                    }
                    11 => {
                        let cur = state.container_table_state.selected().unwrap_or(0);
                        if cur + 1 < state.dynamic_data.containers.len() {
                            state.container_table_state.select(Some(cur + 1));
                        }
                    }
                    _ => {}
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if let Some((_, _, ref mut sel_idx)) = state.signal_modal {
                if *sel_idx > 0 {
                    *sel_idx -= 1;
                } else {
                    *sel_idx = crate::types::POSIX_SIGNALS.len() - 1;
                }
            } else if let Some((_, _, _, ref mut scroll)) = state.viewing_container_logs {
                *scroll = scroll.saturating_sub(3);
            } else {
                match state.active_tab {
                    1 => {
                        if state.selected_pid.is_some() {
                            state.process_detail_scroll = state.process_detail_scroll.saturating_sub(3);
                        } else {
                            handle_process_navigation(&mut state, false);
                        }
                    }
                    2 => {
                        state.cpu_cores_scroll = state.cpu_cores_scroll.saturating_sub(1);
                    }
                    5 => {
                        state.network_socket_scroll = state.network_socket_scroll.saturating_sub(1);
                    }
                    8 => {
                        if state.services_subtab == 0 {
                            let cur = state.services_table_state.selected().unwrap_or(0);
                            state.services_table_state.select(Some(cur.saturating_sub(1)));
                        } else {
                            let cur = state.timers_table_state.selected().unwrap_or(0);
                            state.timers_table_state.select(Some(cur.saturating_sub(1)));
                        }
                    }
                    9 => {
                        let cur = state.logs_table_state.selected().unwrap_or(0);
                        state.logs_table_state.select(Some(cur.saturating_sub(1)));
                    }
                    10 => {
                        let cur = state.config_table_state.selected().unwrap_or(0);
                        state.config_table_state.select(Some(cur.saturating_sub(1)));
                    }
                    11 => {
                        let cur = state.container_table_state.selected().unwrap_or(0);
                        state.container_table_state.select(Some(cur.saturating_sub(1)));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    Ok(())
}

async fn data_collection_loop(
    app_state: Arc<Mutex<AppState>>,
    data_collector: SharedDataCollector,
    _config: AppConfig,
) {
    let mut prev_global_usage = types::GlobalUsage::default();
    
    loop {
        let is_paused = {
            let state = app_state.lock().unwrap();
            state.paused
        };
        
        if is_paused {
            sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        let collection_start = Instant::now();
        
        let (selected_pid, show_system_processes, filter_text, sort_by, sort_ascending, active_tab, tree_mode, current_refresh_ms) = {
            let state = app_state.lock().unwrap();
            (
                state.selected_pid,
                state.show_system_processes,
                state.filter_text.clone(),
                state.sort_by.clone(),
                state.sort_ascending,
                state.active_tab,
                state.process_tree_mode,
                state.refresh_rate_ms,
            )
        };
        
        let new_data = {
            let mut collector = data_collector.lock().await;
            collector.collect_data(
                selected_pid,
                show_system_processes,
                &filter_text,
                &sort_by,
                sort_ascending,
                tree_mode,
                prev_global_usage.clone(),
                active_tab,
            ).await
        };
        
        prev_global_usage = new_data.global_usage.clone();
        
        {
            let mut state = app_state.lock().unwrap();
            state.dynamic_data = new_data;
            
            if state.process_table_state.selected().is_none() && !state.dynamic_data.processes.is_empty() {
                state.process_table_state.select(Some(0));
            }
            if state.container_table_state.selected().is_none() && !state.dynamic_data.containers.is_empty() {
                state.container_table_state.select(Some(0));
            }
        }
        
        let collection_end = Instant::now();
        let collection_duration = collection_end.duration_since(collection_start);
        
        if collection_duration > Duration::from_millis(current_refresh_ms / 2) {
            log::warn!("Slow data collection: {:?}", collection_duration);
        }
        
        let remaining_time = Duration::from_millis(current_refresh_ms.max(100)).saturating_sub(collection_duration);
        if remaining_time > Duration::from_millis(10) {
            sleep(remaining_time).await;
        } else {
            sleep(Duration::from_millis(10)).await;
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



fn check_system_requirements() -> Result<(), AppError> {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        if let Ok((width, height)) = crossterm::terminal::size() {
            if width < 80 || height < 24 {
                log::warn!("Warning: Terminal size {}x{} is smaller than recommended 80x24", width, height);
            }
        }
        return Ok(());
    }

    // okiedokie
    #[cfg(unix)]
    {
        use std::process::Command;
        use std::env;

        if let Ok(exe_path) = env::current_exe() {
            let terminals = [
                ("x-terminal-emulator", "-e"),
                ("gnome-terminal", "--"),
                ("konsole", "-e"),
                ("xfce4-terminal", "-e"),
                ("lxterminal", "-e"),
                ("mate-terminal", "-e"),
                ("kitty", "-e"),
                ("alacritty", "-e"),
                ("xterm", "-e"),
            ];

            for (term, arg) in terminals {
                 if Command::new(term)
                    .arg(arg)
                    .arg(&exe_path)
                    .spawn()
                    .is_ok() 
                {
                    std::process::exit(0);
                }
            }
        }
    }

    Err(AppError::Config(
        "PULS is a terminal application. Please run it inside a terminal emulator.".to_string()
    ))
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

    #[test]
    fn test_modify_setting_cycles() {
        let mut state = AppState::default();

        // 0: Refresh rate (500 -> 1000 -> 2000 -> 5000)
        state.settings_selected_idx = 0;
        state.refresh_rate_ms = 1000;
        modify_setting(&mut state, 1);
        assert_eq!(state.refresh_rate_ms, 2000);
        modify_setting(&mut state, 1);
        assert_eq!(state.refresh_rate_ms, 5000);
        modify_setting(&mut state, 1);
        assert_eq!(state.refresh_rate_ms, 500);
        modify_setting(&mut state, -1);
        assert_eq!(state.refresh_rate_ms, 5000);

        // 1: Temperature Unit
        state.settings_selected_idx = 1;
        state.temp_unit_fahrenheit = false;
        modify_setting(&mut state, 1);
        assert!(state.temp_unit_fahrenheit);
        modify_setting(&mut state, -1);
        assert!(!state.temp_unit_fahrenheit);

        // 2: Color Theme (0..5)
        state.settings_selected_idx = 2;
        state.current_theme = 0;
        modify_setting(&mut state, 1);
        assert_eq!(state.current_theme, 1);
        modify_setting(&mut state, -1);
        assert_eq!(state.current_theme, 0);
        modify_setting(&mut state, -1);
        assert_eq!(state.current_theme, crate::ui::colors::THEME_COUNT - 1);

        // 3: Default Startup Tab (0..12)
        state.settings_selected_idx = 3;
        state.default_tab = 0;
        modify_setting(&mut state, 1);
        assert_eq!(state.default_tab, 1);
        modify_setting(&mut state, -1);
        assert_eq!(state.default_tab, 0);
        modify_setting(&mut state, -1);
        assert_eq!(state.default_tab, 12);

        // 4: Process Hierarchy
        state.settings_selected_idx = 4;
        state.process_tree_mode = false;
        modify_setting(&mut state, 1);
        assert!(state.process_tree_mode);
        modify_setting(&mut state, 1);
        assert!(!state.process_tree_mode);

        // 5: Interface Language
        state.settings_selected_idx = 5;
        state.language = crate::language::Language::English;
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::Turkish);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::French);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::German);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::Spanish);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::Italian);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::Russian);
        modify_setting(&mut state, 1);
        assert_eq!(state.language, crate::language::Language::English);
        modify_setting(&mut state, -1);
        assert_eq!(state.language, crate::language::Language::Russian);
    }
}