pub mod widgets;
pub mod colors;
pub mod layouts;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Cell, Sparkline, Table, Tabs, BorderType, Chart, Dataset, GraphType, Axis},
    symbols::Marker,
};

use crate::types::AppState;
use crate::utils::{format_size, format_rate, format_percentage, format_count, get_usage_color, truncate_string};
use crate::language::Translator;

pub use layouts::*;

pub fn render_ui(f: &mut Frame, state: &mut AppState, is_safe_mode: bool, translator: &Translator) {
    let theme_manager = crate::ui::colors::ThemeManager::from_index(state.current_theme);
    let theme = theme_manager.get_theme();
    
    let main_layout = create_main_layout(f.size());
    
    render_tab_bar(f, state, main_layout.tab_area, is_safe_mode, translator, theme);
    
    render_summary_bar(f, state, main_layout.summary_area, translator, theme);
    
    match state.active_tab {
        0 => render_dashboard_tab(f, state, main_layout.content_area, translator, theme),
        1 => render_processes_tab(f, state, main_layout.content_area, translator, theme),
        2 => render_cpu_cores_tab(f, state, main_layout.content_area, translator, theme),
        3 => render_memory_tab(f, state, main_layout.content_area, translator, theme),
        4 => render_disks_tab(f, state, main_layout.content_area, translator, theme),
        5 => render_network_tab(f, state, main_layout.content_area, is_safe_mode, translator, theme),
        6 => render_gpu_tab(f, state, main_layout.content_area, is_safe_mode, translator, theme),
        7 => render_system_info_tab(f, state, main_layout.content_area, translator, theme),
        8 => render_services_tab(f, state, main_layout.content_area, translator, theme),
        9 => render_logs_tab(f, state, main_layout.content_area, translator, theme),
        10 => render_config_tab(f, state, main_layout.content_area, translator, theme),
        11 => render_containers_tab(f, state, main_layout.content_area, translator, theme),
        12 => render_sensors_tab(f, state, main_layout.content_area, translator, theme),
        _ => {}
    }
    
    render_footer(f, state, main_layout.footer_area, translator);

    if let Some((name, status)) = &state.service_status_modal {
        render_service_status_modal(f, name, status, translator, theme);
    }
    
    if let Some(pid) = state.pending_kill_pid {
        render_kill_confirmation(f, pid, translator, theme);
    }

    if let Some((pid, name, selected_idx)) = &state.signal_modal {
        render_signal_modal(f, *pid, name, *selected_idx, translator, theme);
    }
    
    if let Some((action, name)) = &state.pending_service_action {
        render_service_action_confirmation(f, action, name, translator, theme);
    }

    if let Some((action, name, id)) = &state.pending_container_action {
        render_container_action_confirmation(f, action, name, id, translator, theme);
    }

    if let Some((name, id, logs, scroll)) = &state.viewing_container_logs {
        render_container_logs_modal(f, name, id, logs, *scroll, translator, theme);
    }
    
    if let Some((idx, new_value)) = &state.pending_config_confirmation {
        if let Some(item) = state.config_items.get(*idx) {
             render_config_confirmation_modal(f, &item.key, &item.value, new_value, translator, theme);
        }
    }
    
    if let Some(log) = &state.viewing_log {
        render_log_details_modal(f, log, translator, theme);
    }

    if state.pending_grub_update_confirmation {
        render_grub_update_modal(f, state, translator, theme);
    }

    if state.show_settings_modal {
        render_settings_modal(f, state, translator, theme);
    }
}

fn render_service_status_modal(f: &mut Frame, name: &str, status: &str, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    let popup_area = Rect {
        x: area.width / 10,
        y: area.height / 10,
        width: area.width * 8 / 10,
        height: area.height * 8 / 10,
    };
    
    f.render_widget(ratatui::widgets::Clear, popup_area);
    
    let block = Block::default()
        .title(translator.t("modal.status_title").replace("{}", name))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.highlight));
            
    let paragraph = Paragraph::new(status)
        .block(block)
        .style(Style::default().fg(theme.text))
        .wrap(ratatui::widgets::Wrap { trim: false });
        
    f.render_widget(paragraph, popup_area);
}

fn render_kill_confirmation(f: &mut Frame, pid: sysinfo::Pid, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 2,
        width: area.width / 2,
        height: 5,
    };
    
    f.render_widget(ratatui::widgets::Clear, popup_area);
    
    let block = Block::default()
        .title(translator.t("modal.kill_title"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning));
    
    let text = translator.t("modal.kill_confirm").replace("{}", &pid.to_string()).replace("\\n", "\n");
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Center);
        
    f.render_widget(paragraph, popup_area);
}

fn render_signal_modal(
    f: &mut Frame,
    pid: sysinfo::Pid,
    name: &str,
    selected_idx: usize,
    translator: &Translator,
    theme: &crate::ui::colors::ColorScheme,
) {
    let area = f.size();
    let width = 66.min(area.width.saturating_sub(4));
    let height = 14.min(area.height.saturating_sub(2));
    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let signals = crate::types::POSIX_SIGNALS;
    let mut lines = Vec::new();
    let target_text = translator.t("modal.signal_target").replace("{}", name).replacen("{}", &pid.to_string(), 1);
    lines.push(Line::from(vec![
        Span::styled(target_text, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(translator.t("modal.signal_help"), Style::default().fg(theme.text_secondary)),
    ]));
    lines.push(Line::from(""));

    for (i, sig) in signals.iter().enumerate() {
        let is_selected = i == selected_idx;
        let prefix = if is_selected { " >> " } else { "    " };
        let style = if is_selected {
            Style::default().fg(theme.highlight).bg(theme.border).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        let sig_desc = match sig.name {
            "SIGTERM" => translator.t("sig.term"),
            "SIGKILL" => translator.t("sig.kill"),
            "SIGHUP" => translator.t("sig.hup"),
            "SIGSTOP" => translator.t("sig.stop"),
            "SIGCONT" => translator.t("sig.cont"),
            "SIGINT" => translator.t("sig.int"),
            _ => sig.desc.to_string(),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<17} ", format!("{}{}", prefix, sig.name)), style),
            Span::styled(format!("- {}", sig_desc), if is_selected { style } else { Style::default().fg(theme.text_secondary) }),
        ]));
    }

    let block = Block::default()
        .title(format!(" [*] {}: {} ", translator.t("modal.signal_title"), pid))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning).add_modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn render_service_action_confirmation(f: &mut Frame, action: &str, name: &str, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 2,
        width: area.width / 2,
        height: 5,
    };

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let action_key = match action.to_lowercase().as_str() {
        "start" => "action.start",
        "stop" => "action.stop",
        "restart" => "action.restart",
        "reload" => "action.reload",
        "enable" => "action.enable",
        "disable" => "action.disable",
        _ => "",
    };
    let action_display = if !action_key.is_empty() { translator.t(action_key) } else { action.to_string() };

    let title = translator.t("modal.service_title").replace("{}", &action_display.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning));

    let text = translator.t("modal.service_confirm").replace("{}", &action_display).replacen("{}", name, 1).replace("\\n", "\n");
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, popup_area);
}

fn render_container_action_confirmation(f: &mut Frame, action: &str, name: &str, id: &str, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2 - 3,
        width: area.width / 2,
        height: 6,
    };

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let action_key = match action.to_lowercase().as_str() {
        "start" => "action.start",
        "stop" => "action.stop",
        "restart" => "action.restart",
        _ => "",
    };
    let action_display = if !action_key.is_empty() { translator.t(action_key) } else { action.to_string() };

    let title = translator.t("modal.container_title").replace("{}", &action_display.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning));

    let text = translator.t("modal.container_confirm").replace("{}", &action_display.to_uppercase()).replacen("{}", name, 1).replacen("{}", id, 1).replace("\\n", "\n");
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, popup_area);
}

fn render_container_logs_modal(f: &mut Frame, name: &str, id: &str, logs: &[String], scroll: usize, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    let popup_area = Rect {
        x: area.width / 10,
        y: area.height / 10,
        width: area.width * 8 / 10,
        height: area.height * 8 / 10,
    };

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let title_text = format!(" {} ", translator.t("modal.container_logs_title").replace("{}", name).replacen("{}", id, 1));
    let block = Block::default()
        .title(title_text)
        .title(
            ratatui::widgets::block::Title::from(" [↑/↓/PgUp/PgDn] Scroll ")
                .alignment(Alignment::Right),
        )
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.highlight));

    let inner_area = block.inner(popup_area);
    let visible_height = inner_area.height as usize;
    let total_lines = logs.len();

    let display_text = if logs.is_empty() {
        translator.t("msg.no_logs")
    } else {
        let max_scroll = total_lines.saturating_sub(visible_height);
        let effective_scroll = scroll.min(max_scroll);
        let slice = &logs[effective_scroll..];
        slice.join("\n")
    };

    let paragraph = Paragraph::new(display_text)
        .block(block)
        .style(Style::default().fg(theme.text))
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

pub fn get_tab_at_column(col: u16, translator: &Translator) -> Option<usize> {
    let tab_keys = [
        "tab.dashboard", "tab.process", "tab.cpu", "tab.memory", "tab.disks",
        "tab.network", "tab.gpu", "tab.system", "tab.services", "tab.logs",
        "tab.config", "tab.containers", "tab.sensors"
    ];
    let mut current_x = 1u16;
    for (i, &key) in tab_keys.iter().enumerate() {
        let title = translator.t(key);
        let title_len = title.chars().count() as u16;
        let tab_end = current_x + title_len;
        if col >= current_x && col < tab_end {
            return Some(i);
        }
        current_x = tab_end + 3; // 3 for " | " divider
    }
    None
}

fn render_settings_modal(
    f: &mut Frame,
    state: &AppState,
    translator: &Translator,
    theme: &crate::ui::colors::ColorScheme,
) {
    let area = f.size();
    let width = 68.min(area.width.saturating_sub(4));
    let height = 20.min(area.height.saturating_sub(2));
    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let items = [
        translator.t("settings.refresh_interval"),
        translator.t("settings.temp_units"),
        translator.t("settings.active_theme"),
        translator.t("settings.default_tab"),
        translator.t("settings.process_hierarchy"),
        translator.t("settings.interface_language"),
    ];

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(translator.t("settings.auto_save"), Style::default().fg(theme.text_secondary)),
    ]));
    lines.push(Line::raw(""));

    let refresh_options = [500, 1000, 2000, 5000];
    let tab_keys = [
        "tab.dashboard", "tab.process", "tab.cpu", "tab.memory", "tab.disks",
        "tab.network", "tab.gpu", "tab.system", "tab.services", "tab.logs",
        "tab.config", "tab.containers", "tab.sensors"
    ];
    let tab_num_prefixes = ["1:", "2:", "3:", "4:", "5:", "6:", "7:", "8:", "9:", "0:", "-:", "=:", "+:"];

    for (idx, item_name) in items.iter().enumerate() {
        let is_selected = idx == state.settings_selected_idx;
        let prefix = if is_selected { " >> " } else { "    " };

        let val_str = match idx {
            0 => {
                let cur = state.refresh_rate_ms;
                let formatted: Vec<String> = refresh_options.iter().map(|&r| {
                    if r == cur { format!("<{}ms>", r) } else { format!(" {}ms ", r) }
                }).collect();
                formatted.join(" ")
            }
            1 => {
                if state.temp_unit_fahrenheit {
                    "[ Celsius (°C) ]   < Fahrenheit (°F) >".to_string()
                } else {
                    "< Celsius (°C) >   [ Fahrenheit (°F) ]".to_string()
                }
            }
            2 => {
                let name = crate::ui::colors::ThemeManager::theme_name(state.current_theme);
                format!("< {} > ({}/{})", name, (state.current_theme % crate::ui::colors::THEME_COUNT) + 1, crate::ui::colors::THEME_COUNT)
            }
            3 => {
                let prefix_str = tab_num_prefixes.get(state.default_tab).copied().unwrap_or("1:");
                let key_str = tab_keys.get(state.default_tab).copied().unwrap_or("tab.dashboard");
                let tab_name = format!("{}{}", prefix_str, translator.t(key_str));
                format!("< {} >", tab_name)
            }
            4 => {
                if state.process_tree_mode {
                    format!("< {} (t) >   [ {} ]", translator.t("settings.tree_hierarchy"), translator.t("settings.flat_list"))
                } else {
                    format!("[ {} ]   < {} (t) >", translator.t("settings.tree_hierarchy"), translator.t("settings.flat_list"))
                }
            }
            5 => {
                let cur_name = state.language.name();
                let cur_idx = match state.language {
                    crate::language::Language::English => 1,
                    crate::language::Language::Turkish => 2,
                    crate::language::Language::French => 3,
                    crate::language::Language::German => 4,
                    crate::language::Language::Spanish => 5,
                    crate::language::Language::Italian => 6,
                    crate::language::Language::Russian => 7,
                };
                format!("< {} > ({}/7)", cur_name, cur_idx)
            }
            _ => String::new(),
        };

        let label_style = if is_selected {
            Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let val_style = if is_selected {
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_secondary)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<22}: ", prefix, item_name), label_style),
            Span::styled(val_str, val_style),
        ]));
        lines.push(Line::raw(""));
    }

    lines.push(Line::from(vec![
        Span::styled(translator.t("settings.help"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
    ]));

    let block = Block::default()
        .title(format!(" [*] {} ", translator.t("settings.title")))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn render_tab_bar(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let tab_titles: Vec<Line> = [
        "tab.dashboard", "tab.process", "tab.cpu", "tab.memory", "tab.disks",
        "tab.network", "tab.gpu", "tab.system", "tab.services", "tab.logs",
        "tab.config", "tab.containers", "tab.sensors"
    ]
    .iter()
    .enumerate()
    .map(|(i, &key)| {
        let title = translator.t(key);
        let style = if is_safe_mode && (i == 5 || i == 6 || i == 8 || i == 9 || i == 10) {
            Style::default().fg(theme.text_secondary)
        } else if i == state.active_tab {
            Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        Line::from(Span::styled(title, style))
    })
    .collect();

    let right_title = format!(
        " [{}] [L: {}] v{} ",
        translator.t("help.settings_badge"),
        state.language.code().to_uppercase(),
        env!("CARGO_PKG_VERSION")
    );

    let tabs = Tabs::new(tab_titles)
        .block(Block::default()
            .title(translator.t("title.puls"))
            .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
            .title(ratatui::widgets::block::Title::from(right_title).alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)))
        .select(state.active_tab)
        .highlight_style(Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, area);
}

fn render_summary_bar(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let usage = &state.dynamic_data.global_usage;
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // CPU
            Constraint::Percentage(25), // Memory
            Constraint::Percentage(15), // GPU
            Constraint::Percentage(20), // Network
            Constraint::Percentage(20), // Disk I/O
        ])
        .split(area);
    
    let cpu_temp = state.dynamic_data.temperatures.cpu_temp;
    
    let mem_temp = state.dynamic_data.sensors.iter()
        .find(|s| {
            let label = s.label.to_lowercase();
            label.contains("dimm") || label.contains("dram") || label.contains("memory")
        })
        .map(|s| s.temp);

    render_cpu_gauge(f, usage.cpu, usage.load_average, cpu_temp, layout[0], translator, theme, state.temp_unit_fahrenheit);
    
    render_memory_gauge(f, usage.mem_used, usage.mem_total, mem_temp, layout[1], translator, theme, state.temp_unit_fahrenheit);
    
    render_gpu_gauge(f, usage.gpu_util, layout[2], translator, theme);
    
    render_network_summary(f, usage, layout[3], translator, theme);
    
    render_disk_summary(f, usage, layout[4], translator, theme);
}

#[allow(clippy::too_many_arguments)]
fn render_cpu_gauge(f: &mut Frame, cpu_percent: f32, load_avg: (f64, f64, f64), temp: Option<f32>, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme, fahrenheit: bool) {
    let color = get_usage_color(cpu_percent);
    let temp_str = temp.map(|t| format!(" | {}", crate::utils::format_temp_int(t, fahrenheit))).unwrap_or_default();
    let label = format!("{:.1}%{} | {}: {:.1}", cpu_percent, temp_str, translator.t("label.load"), load_avg.0);
    let gauge = Gauge::default()
        .block(Block::default()
            .title(translator.t("title.cpu"))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)))
        .gauge_style(Style::default().fg(color))
        .percent(cpu_percent.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

#[allow(clippy::too_many_arguments)]
fn render_memory_gauge(f: &mut Frame, mem_used: u64, mem_total: u64, temp: Option<f32>, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme, fahrenheit: bool) {
    let mem_percent = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let color = get_usage_color(mem_percent as f32);
    
    let pressure = match mem_percent {
        x if x >= 90.0 => "health.critical",
        x if x >= 80.0 => "health.high",
        x if x >= 60.0 => "health.moderate",
        _ => "health.healthy",
    };
    
    let temp_str = temp.map(|t| format!(" | {}", crate::utils::format_temp_int(t, fahrenheit))).unwrap_or_default();
    let label = format!("{} ({}: {}%){}", format_size(mem_used), translator.t(pressure), mem_percent as u16, temp_str);
    
    let gauge = Gauge::default()
        .block(Block::default()
            .title(translator.t("title.memory"))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)))
        .gauge_style(Style::default().fg(color))
        .percent(mem_percent.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

fn render_gpu_gauge(f: &mut Frame, gpu_util: Option<u32>, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let block = Block::default()
        .title(translator.t("title.gpu"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    
    if let Some(gpu_percent) = gpu_util {
        let color = get_usage_color(gpu_percent as f32);
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color))
            .percent(gpu_percent.clamp(0, 100) as u16)
            .label(format!("{}%", gpu_percent));
        f.render_widget(gauge, area);
    } else {
        let paragraph = Paragraph::new("N/A")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_secondary))
            .block(block);
        f.render_widget(paragraph, area);
    }
}

fn render_network_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let block = Block::default()
        .title(translator.t("title.network"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);
    
    let net_text = format!("v{} ^{}", format_rate(usage.net_down), format_rate(usage.net_up));
    let net_paragraph = Paragraph::new(net_text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(theme.accent));
    f.render_widget(net_paragraph, layout[0]);
    
    if !usage.net_down_history.is_empty() {
         let data: Vec<u64> = usage.net_down_history.iter().cloned().collect();
         let sparkline = Sparkline::default()
            .data(&data)
            .style(Style::default().fg(theme.accent));
        f.render_widget(sparkline, layout[1]);
    }
}

fn render_disk_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let block = Block::default()
        .title(translator.t("title.disk"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    let disk_text = format!("R:{} W:{}", format_rate(usage.disk_read), format_rate(usage.disk_write));
    let disk_paragraph = Paragraph::new(disk_text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(theme.warning));
    f.render_widget(disk_paragraph, layout[0]);
    
    if !usage.disk_read_history.is_empty() {
        let data: Vec<u64> = usage.disk_read_history.iter().cloned().collect();
        let sparkline = Sparkline::default()
             .data(&data)
             .style(Style::default().fg(theme.warning));
        f.render_widget(sparkline, layout[1]);
    }
}

fn render_dashboard_cpu_chart(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let history_data: Vec<(f64, f64)> = state.dynamic_data.global_usage.cpu_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(translator.t("cpu.total_usage"))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.primary))
            .data(&history_data)
    ];
    
    let chart = Chart::new(datasets)
        .block(Block::default()
            .title(format!(" {} ", translator.t("dashboard.cpu_history")))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(Axis::default()
            .bounds([0.0, 100.0])
            .labels(vec![
                Span::styled("0%", Style::default().fg(theme.text_secondary)),
                Span::styled("100%", Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().fg(theme.text_secondary)));
    f.render_widget(chart, area);
}

fn render_dashboard_mem_chart(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let history_data: Vec<(f64, f64)> = state.dynamic_data.global_usage.mem_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(translator.t("title.memory"))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.accent))
            .data(&history_data)
    ];
    
    let chart = Chart::new(datasets)
        .block(Block::default()
            .title(format!(" {} ", translator.t("dashboard.mem_history")))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(Axis::default()
            .bounds([0.0, 100.0])
            .labels(vec![
                Span::styled("0%", Style::default().fg(theme.text_secondary)),
                Span::styled("100%", Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().fg(theme.text_secondary)));
    f.render_widget(chart, area);
}

fn render_top_processes(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let mut processes = state.dynamic_data.processes.clone();
    processes.truncate(5);

    let header_name = translator.t("header.name");
    let header_cpu = translator.t("header.cpu");
    let header_memory = translator.t("header.memory");

    let rows = processes.iter().map(|p| {
        Row::new(vec![
            truncate_string(&p.name, 25),
            format!("{:.2}%", p.cpu),
            crate::utils::format_size(p.mem),
        ]).style(Style::default().fg(theme.text))
    });

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),     // Name
            Constraint::Length(8),   // CPU
            Constraint::Length(10),  // Memory
        ]
    )
    .header(
        Row::new(vec![header_name, header_cpu, header_memory])
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(
        Block::default()
            .title(format!(" {} ", translator.t("dashboard.top_processes")))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );

    f.render_widget(table, area);
}

fn render_dashboard_storage(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let disks = &state.dynamic_data.disks;
    let block = Block::default()
        .title(format!(" {} ", translator.t("dashboard.storage")))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let disk_count = disks.len().min(3);
    if disk_count == 0 { return; }

    let constraints: Vec<Constraint> = (0..disk_count).map(|_| Constraint::Length(3)).collect();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (i, disk) in disks.iter().take(disk_count).enumerate() {
        let usage_percent = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        let label = format!("{} ({:.1}%)", disk.name, usage_percent);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(get_usage_color(usage_percent)))
            .percent(usage_percent as u16)
            .label(label);
        f.render_widget(gauge, layout[i]);
    }
}

fn render_dashboard_tab(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // System Status
            Constraint::Length(5),      // Diagnostics Panel
            Constraint::Percentage(40), // Charts
            Constraint::Min(10),        // Tables
        ])
        .split(area);
    
    render_system_status(f, state, chunks[0], translator, theme);
    render_dashboard_diagnostics(f, state, chunks[1], translator, theme);
    
    let chart_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    
    render_dashboard_cpu_chart(f, state, chart_chunks[0], translator, theme);
    render_dashboard_mem_chart(f, state, chart_chunks[1], translator, theme);
    
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(35), Constraint::Percentage(30)])
        .split(chunks[3]);
    
    render_top_processes(f, state, bottom_chunks[0], translator, theme);
    render_dashboard_storage(f, state, bottom_chunks[1], translator, theme);
    render_container_table(f, state, bottom_chunks[2], translator, theme);
}

fn render_dashboard_diagnostics(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let mut alerts = Vec::new();
    
    let failed_services: Vec<String> = state.services.iter()
        .filter(|s| s.status == "Failed")
        .map(|s| s.name.clone())
        .collect();
    if !failed_services.is_empty() {
        alerts.push(format!("{}: {}", translator.t("msg.failed_service"), failed_services.join(", ")));
    }
    
    let critical_disks: Vec<String> = state.dynamic_data.disks.iter()
        .filter(|d| d.total > 0 && (d.used as f64 / d.total as f64) > 0.90)
        .map(|d| format!("{} ({:.0}%)", d.name, (d.used as f64 / d.total as f64) * 100.0))
        .collect();
    if !critical_disks.is_empty() {
        alerts.push(format!("{}: {}", translator.t("msg.critical_storage"), critical_disks.join(", ")));
    }
    
    if let Some(temp) = state.dynamic_data.temperatures.cpu_temp {
        if temp > 80.0 {
            alerts.push(format!("{}: {:.0}°C", translator.t("msg.high_cpu_temp"), temp));
        }
    }
    
    let usage = &state.dynamic_data.global_usage;
    let mem_percent = if usage.mem_total > 0 {
        (usage.mem_used as f64 / usage.mem_total as f64) * 100.0
    } else {
        0.0
    };
    if mem_percent > 90.0 {
        alerts.push(format!("{}: {:.0}%", translator.t("msg.high_mem_pressure"), mem_percent));
    }
    
    let (widget_text, block_style, text_color) = if alerts.is_empty() {
        (
            format!(" * {}: {}", translator.t("title.diagnostics"), translator.t("msg.diagnostics_nominal")),
            Style::default().fg(theme.success),
            theme.success,
        )
    } else {
        (
            format!(" [*] {}: {} alerts active\n {}", 
                translator.t("title.diagnostics"), 
                alerts.len(), 
                alerts.iter().map(|a| format!("- {}", a)).collect::<Vec<_>>().join(" | ")
            ),
            Style::default().fg(theme.error),
            theme.error,
        )
    };
    
    let diagnostics_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(block_style);
        
    let paragraph = Paragraph::new(widget_text)
        .block(diagnostics_block)
        .style(Style::default().fg(text_color))
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });
        
    f.render_widget(paragraph, area);
}

fn render_processes_tab(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    if state.selected_pid.is_some() {
        render_process_detail_tab(f, state, area, translator, theme);
    } else {
        render_process_table(f, state, area, translator, theme);
    }
}

fn render_system_status(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let usage = &state.dynamic_data.global_usage;
    let system_info = &state.system_info;
    
    let cpu_cores = system_info.iter()
        .find(|(k, _)| k == "Cores")
        .and_then(|(_, v)| v.split_whitespace().next()?.parse::<usize>().ok())
        .unwrap_or(1);
    
    let (status_str, load_per_core) = crate::utils::get_system_health_localized(
        usage.load_average.0,
        cpu_cores,
        usage.mem_used,
        usage.mem_total,
        translator,
    );
    
    let mem_percent = if usage.mem_total > 0 {
        (usage.mem_used as f64 / usage.mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let cpu_efficiency = crate::utils::get_cpu_efficiency_localized(usage.cpu, usage.load_average.0, cpu_cores, translator);
    let (mem_available, _availability_level) = crate::utils::estimate_memory_availability_localized(usage.mem_used, usage.mem_total, translator);
    
    let cpu_temp = state.dynamic_data.temperatures.cpu_temp;
    let cpu_temp_str = cpu_temp.map(|t| format!(" | {}", crate::utils::format_temp_int(t, state.temp_unit_fahrenheit))).unwrap_or_default();

    let gpu_str = if let Ok(gpus) = &state.dynamic_data.gpus {
        if let Some(gpu) = gpus.first() {
            let gpu_temp_str = crate::utils::format_temp_int(gpu.temperature as f32, state.temp_unit_fahrenheit);
            if gpu.is_throttling {
                format!(" | GPU: {}% ({} [{}])", gpu.utilization, gpu_temp_str, translator.t("overview.throttled"))
            } else {
                format!(" | GPU: {}% ({})", gpu.utilization, gpu_temp_str)
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let bat_str = if let Some(bat) = &state.dynamic_data.battery {
        let is_charging = bat.status.eq_ignore_ascii_case("charging");
        let sym = if is_charging {
            "+"
        } else if bat.ac_online {
            "="
        } else {
            "-"
        };
        format!(" | BAT: {}% ({}{})", bat.capacity, sym, bat.status)
    } else {
        String::new()
    };

    let reboot_str = if state.dynamic_data.reboot_required {
        format!(" | {}", translator.t("overview.reboot_required"))
    } else {
        String::new()
    };

    let status_text = format!(
        "{} {} | CPU: {:.0}% ({}: {}){}{}{}{} | {}: {:.2}/core | Mem: {:.0}% ({} {}) | Swap: {:.0}% | {}: {} | {}: {}",
        translator.t("overview.status"),
        status_str,
        usage.cpu,
        translator.t("overview.eff"),
        cpu_efficiency,
        cpu_temp_str,
        gpu_str,
        bat_str,
        reboot_str,
        translator.t("overview.load"),
        load_per_core.parse::<f64>().unwrap_or(0.0),
        mem_percent,
        format_size(mem_available),
        translator.t("overview.free"),
        if usage.swap_total > 0 { (usage.swap_used as f64 / usage.swap_total as f64) * 100.0 } else { 0.0 },
        translator.t("overview.up"),
        crate::utils::format_uptime(usage.uptime),
        translator.t("overview.procs"),
        state.dynamic_data.processes.len()
    );
    
    let (border_color, title_suffix) = if state.dynamic_data.reboot_required {
        (theme.warning, format!(" {}", translator.t("overview.reboot_required")))
    } else {
        (theme.success, String::new())
    };

    let status_paragraph = Paragraph::new(status_text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(theme.text))
        .block(
            Block::default()
                .title(format!("{}{}", translator.t("title.system_overview"), title_suffix))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
        );
    
    f.render_widget(status_paragraph, area);
}

fn render_process_table(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let processes = &state.dynamic_data.processes;
    let header_pid = translator.t("header.pid");
    let header_name = translator.t("header.name");
    let header_user = translator.t("header.user");
    let header_nice = "NI".to_string();
    let header_cpu = translator.t("header.cpu");
    let header_memory = translator.t("header.memory");
    let header_disk_read = translator.t("header.disk_read");
    let header_disk_write = translator.t("header.disk_write");
    
    let chunks = if state.editing_filter || !state.filter_text.is_empty() {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Filter input
                Constraint::Min(0),    // Table
            ])
            .split(area)
    } else {
        std::rc::Rc::new([area])
    };

    let table_area = if state.editing_filter || !state.filter_text.is_empty() {
        let filter_title = translator.t("title.process_filter");
        let filter_prompt = if state.editing_filter {
            format!("{}█", state.edit_buffer)
        } else {
            state.filter_text.clone()
        };
        
        let filter_style = if state.editing_filter {
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.success)
        };
        
        let filter_widget = Paragraph::new(filter_prompt)
            .style(filter_style)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", filter_title))
                .border_style(Style::default().fg(if state.editing_filter { theme.primary } else { theme.border })));
        
        f.render_widget(filter_widget, chunks[0]);
        chunks[1]
    } else {
        chunks[0]
    };

    let rows = processes.iter().map(|p| {
        let display_name = if state.process_tree_mode && !p.tree_prefix.is_empty() {
            format!("{}{}", p.tree_prefix, p.name)
        } else {
            p.name.clone()
        };
        Row::new(vec![
            p.pid.clone(),
            truncate_string(&display_name, 35),
            truncate_string(&p.user, 10),
            format!("{:>3}", p.nice),
            format!("{:.2}%", p.cpu),
            crate::utils::format_size(p.mem),
            crate::utils::format_rate(p.disk_read),
            crate::utils::format_rate(p.disk_write),
        ]).style(Style::default().fg(theme.text))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),   // PID
            Constraint::Min(20),     // Name
            Constraint::Length(10),  // User
            Constraint::Length(5),   // NI
            Constraint::Length(8),   // CPU
            Constraint::Length(10),  // Memory
            Constraint::Length(11),  // Read/s
            Constraint::Length(11),  // Write/s
        ]
    )
    .header(
        Row::new(vec![header_pid, header_name, header_user, header_nice, header_cpu, header_memory, header_disk_read, header_disk_write])
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(
        Block::default()
            .title(if state.process_tree_mode {
                format!(" {} [TREE] ", translator.t("title.processes"))
            } else {
                format!(" {} ", translator.t("title.processes"))
            })
            .title_style(Style::default().fg(theme.text))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    )
    .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ");
    
    f.render_stateful_widget(table, table_area, &mut state.process_table_state);
}

fn render_container_table(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let containers = &state.dynamic_data.containers;
    
    if containers.is_empty() {
        let message = if state.system_info.iter().any(|(k, v)| k == "Mode" && v.contains("Safe")) {
            translator.t("msg.container_disabled")
        } else {
            translator.t("msg.no_containers")
        };
        
        let paragraph = Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_secondary))
            .block(
                Block::default()
                    .title(translator.t("title.containers"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
            );
        f.render_widget(paragraph, area);
        return;
    }
    
    let h_pid = translator.t("header.pid");
    let h_name = translator.t("header.name");
    let h_status = translator.t("header.status");
    let h_cpu = translator.t("header.cpu");
    let h_mem = translator.t("header.memory");
    let h_net_down = translator.t("containers.net_down");
    let h_net_up = translator.t("containers.net_up");
    let h_disk_r = translator.t("header.disk_read");
    let h_disk_w = translator.t("header.disk_write");
    
    let headers = vec![
        h_pid.as_str(),
        h_name.as_str(),
        h_status.as_str(),
        h_cpu.as_str(),
        h_mem.as_str(),
        h_net_down.as_str(),
        h_net_up.as_str(),
        h_disk_r.as_str(),
        h_disk_w.as_str(),
    ];
    
    let rows = containers.iter().map(|c| {
        Row::new(vec![
            c.id.clone(),
            truncate_string(&c.name, 20),
            c.status.clone(),
            c.cpu.clone(),
            c.mem.clone(),
            c.net_down.clone(),
            c.net_up.clone(),
            c.disk_r.clone(),
            c.disk_w.clone(),
        ]).style(Style::default().fg(theme.text))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),  // ID
            Constraint::Min(15),     // Name
            Constraint::Length(10),  // Status
            Constraint::Length(8),   // CPU
            Constraint::Length(10),  // Memory
            Constraint::Length(10),  // Net Down
            Constraint::Length(10),  // Net Up
            Constraint::Length(10),  // Disk Read
            Constraint::Length(10),  // Disk Write
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title(translator.t("title.containers"))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    f.render_widget(table, area);
}

fn render_process_detail_tab(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    if state.dynamic_data.detailed_process.is_none() {
        let block = Block::default()
            .title(format!(" {} ", translator.t("title.process_details")))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));
        let inner_area = block.inner(area);
        f.render_widget(block, area);
        let message = Paragraph::new(translator.t("msg.loading_process_details"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_secondary));
        f.render_widget(message, inner_area);
        return;
    }

    let process = state.dynamic_data.detailed_process.clone().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Subtab navigation bar
            Constraint::Min(0),    // Active Subtab View
        ])
        .split(area);

    let total_fds_count = process.file_descriptors.unwrap_or(process.fds.len() as u32);
    let subtab_titles = vec![
        Line::from(format!(" {} ", translator.t("proc_detail.overview"))),
        Line::from(format!(" {} ", translator.t("proc_detail.fds").replace("{}", &total_fds_count.to_string()))),
        Line::from(format!(" {} ", translator.t("proc_detail.threads").replace("{}", &process.threads.to_string()))),
        Line::from(format!(" {} ", translator.t("proc_detail.environ").replace("{}", &process.environ.len().to_string()))),
    ];

    let title_str = format!(" {} ", translator.t("proc_detail.title").replace("{}", &process.name).replacen("{}", &process.pid, 1));
    let nav_hint = format!(" {} ", translator.t("proc_detail.nav_hint"));

    let subtabs = Tabs::new(subtab_titles)
        .block(
            Block::default()
                .title(title_str)
                .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                .title(
                    ratatui::widgets::block::Title::from(nav_hint)
                        .alignment(Alignment::Right),
                )
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        )
        .select(state.process_detail_subtab)
        .highlight_style(
            Style::default()
                .fg(theme.highlight)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );

    f.render_widget(subtabs, chunks[0]);
    let content_area = chunks[1];

    match state.process_detail_subtab {
        0 => {
            let overview_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(65), // Info columns
                    Constraint::Percentage(35), // Cores
                ])
                .split(content_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));
            let inner_area = block.inner(overview_chunks[0]);
            f.render_widget(block, overview_chunks[0]);

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ])
                .split(inner_area);

            // Column 0: General Metadata
            let mut info_lines = vec![
                Line::from(vec![
                    Span::styled(format!("{}: ", translator.t("header.pid")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&process.pid, Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{}: ", translator.t("header.name")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&process.name, Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{}: ", translator.t("header.user")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&process.user, Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.nice_priority")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(process.nice.to_string(), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{}: ", translator.t("header.status")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&process.status, Style::default().fg(crate::ui::colors::process_status_color(&process.status))),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.parent_pid")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(process.parent.as_deref().unwrap_or("N/A"), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.started")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&process.start_time, Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.cpu_usage")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:.2}%", process.cpu_usage), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.mem_rss")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format_size(process.memory_rss), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.mem_vms")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format_size(process.memory_vms), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.disk_read")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format_size(process.io_read_bytes), Style::default().fg(theme.text)),
                    Span::styled(format!("  {} ", translator.t("proc_detail.disk_write")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format_size(process.io_write_bytes), Style::default().fg(theme.text)),
                ]),
            ];

            if let Some(ref cwd) = process.cwd {
                info_lines.push(Line::from(vec![
                    Span::styled(format!("{} ", translator.t("proc_detail.cwd")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(truncate_string(cwd, 30), Style::default().fg(theme.text)),
                ]));
            }

            let info_paragraph = Paragraph::new(info_lines)
                .block(Block::default().borders(Borders::NONE))
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(info_paragraph, layout[0]);

            // Column 1: FDs & Threads Preview
            let mut col1_lines = vec![
                Line::from(Span::styled(translator.t("proc_detail.fds_sockets"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Line::from(vec![
                    Span::styled(format!("  {} ", translator.t("proc_detail.total_fds")), Style::default().fg(theme.accent)),
                    Span::styled(process.file_descriptors.map_or("N/A".to_string(), |v| v.to_string()), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {} ", translator.t("proc_detail.active_sockets")), Style::default().fg(theme.accent)),
                    Span::styled(process.sockets_count.map_or("0".to_string(), |v| v.to_string()), Style::default().fg(theme.text)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {} ", translator.t("proc_detail.active_pipes")), Style::default().fg(theme.accent)),
                    Span::styled(process.pipes_count.map_or("0".to_string(), |v| v.to_string()), Style::default().fg(theme.text)),
                ]),
                Line::from(""),
                Line::from(Span::styled(translator.t("proc_detail.fds_preview"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
            ];

            if process.fds.is_empty() {
                col1_lines.push(Line::from(Span::styled(format!("  {}", translator.t("proc_detail.none_or_denied")), Style::default().fg(theme.text_secondary))));
            } else {
                for fd_info in process.fds.iter().take(4) {
                    col1_lines.push(Line::from(vec![
                        Span::styled(format!("  [{}] ", fd_info.fd), Style::default().fg(theme.accent)),
                        Span::styled(truncate_string(&fd_info.target, 28), Style::default().fg(theme.text)),
                    ]));
                }
                if process.fds.len() > 4 {
                    col1_lines.push(Line::from(Span::styled(
                        format!("  {}", translator.t("proc_detail.more_fds").replace("{}", &(process.fds.len() - 4).to_string())),
                        Style::default().fg(theme.highlight),
                    )));
                }
            }

            col1_lines.push(Line::from(""));
            col1_lines.push(Line::from(Span::styled(
                translator.t("proc_detail.threads_preview").replace("{}", &process.threads.to_string()),
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            )));

            if process.thread_list.is_empty() {
                col1_lines.push(Line::from(Span::styled(format!("  {}", translator.t("proc_detail.single_thread")), Style::default().fg(theme.text_secondary))));
            } else {
                for t in process.thread_list.iter().take(3) {
                    col1_lines.push(Line::from(vec![
                        Span::styled(format!("  [{}] ", t.tid), Style::default().fg(theme.accent)),
                        Span::styled(truncate_string(&t.name, 16), Style::default().fg(theme.text)),
                        Span::styled(format!(" ({})", t.status), Style::default().fg(crate::ui::colors::process_status_color(&t.status))),
                    ]));
                }
                if process.thread_list.len() > 3 {
                    col1_lines.push(Line::from(Span::styled(
                        format!("  {}", translator.t("proc_detail.more_threads").replace("{}", &(process.thread_list.len() - 3).to_string())),
                        Style::default().fg(theme.highlight),
                    )));
                }
            }

            let col1_paragraph = Paragraph::new(col1_lines)
                .block(Block::default().borders(Borders::NONE))
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(col1_paragraph, layout[1]);

            // Column 2: Command & Environment Preview (RESTORED!)
            let mut col2_lines = vec![
                Line::from(Span::styled(translator.t("proc_detail.command"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(truncate_string(&process.command, 75), Style::default().fg(theme.text))),
                Line::from(""),
                Line::from(Span::styled(
                    translator.t("proc_detail.environ_preview").replace("{}", &process.environ.len().to_string()),
                    Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
                )),
            ];

            if process.environ.is_empty() {
                col2_lines.push(Line::from(Span::styled(format!("  {}", translator.t("proc_detail.none_or_denied")), Style::default().fg(theme.text_secondary))));
            } else {
                for env in process.environ.iter().take(8) {
                    col2_lines.push(Line::from(Span::styled(
                        format!("  {}", truncate_string(env, 38)),
                        Style::default().fg(theme.text),
                    )));
                }
                if process.environ.len() > 8 {
                    col2_lines.push(Line::from(Span::styled(
                        format!("  {}", translator.t("proc_detail.more_environ").replace("{}", &(process.environ.len() - 8).to_string())),
                        Style::default().fg(theme.highlight),
                    )));
                }
            }

            let col2_paragraph = Paragraph::new(col2_lines)
                .block(Block::default().borders(Borders::NONE))
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(col2_paragraph, layout[2]);

            // CPU Core Usage at bottom
            let cores = &state.dynamic_data.cores;
            let core_block = Block::default()
                .title(format!(" {} ", translator.t("proc_detail.core_usage")))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));

            let core_inner = core_block.inner(overview_chunks[1]);
            f.render_widget(core_block, overview_chunks[1]);

            let cores_per_row = 8;
            let rows_needed = cores.len().div_ceil(cores_per_row);
            if rows_needed > 0 {
                let row_constraints: Vec<Constraint> = (0..rows_needed).map(|_| Constraint::Length(3)).collect();
                let rows_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(row_constraints)
                    .margin(1)
                    .split(core_inner);

                for (row_idx, row_area) in rows_layout.iter().enumerate() {
                    let start_core = row_idx * cores_per_row;
                    if start_core >= cores.len() { break; }

                    let core_constraints: Vec<Constraint> = (0..cores_per_row).map(|_| Constraint::Ratio(1, cores_per_row as u32)).collect();
                    let cores_layout = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(core_constraints)
                        .split(*row_area);

                    for (core_idx, core_area) in cores_layout.iter().enumerate() {
                        let actual_core_idx = start_core + core_idx;
                        if actual_core_idx >= cores.len() { break; }

                        let core = &cores[actual_core_idx];
                        let gauge = Gauge::default()
                            .block(Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).border_style(Style::default().fg(theme.border)))
                            .label(format!("C{} {:.0}%", actual_core_idx, core.usage))
                            .gauge_style(Style::default().fg(get_usage_color(core.usage)))
                            .ratio((core.usage / 100.0) as f64);
                        f.render_widget(gauge, *core_area);
                    }
                }
            }
        }

        1 => {
            // Subtab 1: Open FDs & Sockets (Full Table)
            let total_fds = process.fds.len();
            let base_block = Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded);
            let inner = base_block.inner(content_area);

            let visible_rows = inner.height.saturating_sub(2) as usize;
            let max_scroll = total_fds.saturating_sub(visible_rows);
            if state.process_detail_scroll > max_scroll {
                state.process_detail_scroll = max_scroll;
            }
            let scroll = state.process_detail_scroll;
            let start_item = if total_fds == 0 { 0 } else { scroll + 1 };
            let end_item = (scroll + visible_rows).min(total_fds);

            let open_fds_title = translator.t("proc_detail.open_fds_title")
                .replace("{}", &process.file_descriptors.unwrap_or(total_fds as u32).to_string())
                .replacen("{}", &process.sockets_count.unwrap_or(0).to_string(), 1)
                .replacen("{}", &process.pipes_count.unwrap_or(0).to_string(), 1);

            let showing_title = translator.t("proc_detail.showing")
                .replace("{}", &start_item.to_string())
                .replacen("{}", &end_item.to_string(), 1)
                .replacen("{}", &total_fds.to_string(), 1);

            let block = Block::default()
                .title(format!(" {} ", open_fds_title))
                .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                .title(
                    ratatui::widgets::block::Title::from(format!(" {} ", showing_title))
                        .alignment(Alignment::Right),
                )
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));

            f.render_widget(block, content_area);

            let rows: Vec<Row> = process.fds.iter().skip(scroll).take(visible_rows).map(|fd_info| {
                let type_color = match fd_info.fd_type.as_str() {
                    "Socket" => theme.warning,
                    "Pipe" => theme.secondary,
                    "Device" => theme.accent,
                    "AnonInode" => theme.text_secondary,
                    _ => theme.text,
                };
                Row::new(vec![
                    Cell::from(Span::styled(&fd_info.fd, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(&fd_info.fd_type, Style::default().fg(type_color))),
                    Cell::from(Span::styled(&fd_info.target, Style::default().fg(theme.text))),
                ])
            }).collect();

            let header = Row::new(vec![
                Cell::from(Span::styled(translator.t("proc_detail.header_fd"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(translator.t("proc_detail.header_type"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(translator.t("proc_detail.header_target"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
            ]).bottom_margin(1);

            let widths = [
                Constraint::Length(8),
                Constraint::Length(14),
                Constraint::Min(20),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(Block::default().borders(Borders::NONE));

            f.render_widget(table, inner);
        }

        2 => {
            // Subtab 2: Process Threads (Full Table)
            let total_threads = process.thread_list.len();
            let base_block = Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded);
            let inner = base_block.inner(content_area);

            let visible_rows = inner.height.saturating_sub(2) as usize;
            let max_scroll = total_threads.saturating_sub(visible_rows);
            if state.process_detail_scroll > max_scroll {
                state.process_detail_scroll = max_scroll;
            }
            let scroll = state.process_detail_scroll;
            let start_item = if total_threads == 0 { 0 } else { scroll + 1 };
            let end_item = (scroll + visible_rows).min(total_threads);

            let threads_title = translator.t("proc_detail.threads_title")
                .replace("{}", &total_threads.to_string());

            let showing_title = translator.t("proc_detail.showing")
                .replace("{}", &start_item.to_string())
                .replacen("{}", &end_item.to_string(), 1)
                .replacen("{}", &total_threads.to_string(), 1);

            let block = Block::default()
                .title(format!(" {} ", threads_title))
                .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                .title(
                    ratatui::widgets::block::Title::from(format!(" {} ", showing_title))
                        .alignment(Alignment::Right),
                )
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));

            f.render_widget(block, content_area);

            let rows: Vec<Row> = process.thread_list.iter().skip(scroll).take(visible_rows).map(|t| {
                let status_color = crate::ui::colors::process_status_color(&t.status);
                Row::new(vec![
                    Cell::from(Span::styled(&t.tid, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(&t.name, Style::default().fg(theme.text))),
                    Cell::from(Span::styled(&t.status, Style::default().fg(status_color))),
                ])
            }).collect();

            let header = Row::new(vec![
                Cell::from(Span::styled(translator.t("proc_detail.header_tid"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(translator.t("proc_detail.header_comm"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(translator.t("proc_detail.header_status"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
            ]).bottom_margin(1);

            let widths = [
                Constraint::Length(12),
                Constraint::Length(35),
                Constraint::Min(15),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(Block::default().borders(Borders::NONE));

            f.render_widget(table, inner);
        }

        _ => {
            // Subtab 3: Environment Variables (Full Table)
            let total_env = process.environ.len();
            let base_block = Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded);
            let inner = base_block.inner(content_area);

            let visible_rows = inner.height.saturating_sub(2) as usize;
            let max_scroll = total_env.saturating_sub(visible_rows);
            if state.process_detail_scroll > max_scroll {
                state.process_detail_scroll = max_scroll;
            }
            let scroll = state.process_detail_scroll;
            let start_item = if total_env == 0 { 0 } else { scroll + 1 };
            let end_item = (scroll + visible_rows).min(total_env);

            let environ_title = translator.t("proc_detail.environ_title")
                .replace("{}", &total_env.to_string());

            let showing_title = translator.t("proc_detail.showing")
                .replace("{}", &start_item.to_string())
                .replacen("{}", &end_item.to_string(), 1)
                .replacen("{}", &total_env.to_string(), 1);

            let block = Block::default()
                .title(format!(" {} ", environ_title))
                .title_style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
                .title(
                    ratatui::widgets::block::Title::from(format!(" {} ", showing_title))
                        .alignment(Alignment::Right),
                )
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border));

            f.render_widget(block, content_area);

            let rows: Vec<Row> = process.environ.iter().skip(scroll).take(visible_rows).map(|env_str| {
                let (var, val) = match env_str.split_once('=') {
                    Some((k, v)) => (k, v),
                    None => (env_str.as_str(), ""),
                };
                Row::new(vec![
                    Cell::from(Span::styled(var, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(val, Style::default().fg(theme.text))),
                ])
            }).collect();

            let header = Row::new(vec![
                Cell::from(Span::styled(translator.t("proc_detail.header_var"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(translator.t("proc_detail.header_val"), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))),
            ]).bottom_margin(1);

            let widths = [
                Constraint::Length(32),
                Constraint::Min(30),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(Block::default().borders(Borders::NONE));

            f.render_widget(table, inner);
        }
    }
}

fn render_cpu_cores_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    use ratatui::widgets::{Chart, Dataset, Axis, Paragraph};
    use ratatui::layout::{Layout, Constraint, Direction};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Modifier};
    use ratatui::widgets::{Block, Borders, BorderType};

    let cores = &state.dynamic_data.cores;
    
    if cores.is_empty() {
        let message = Paragraph::new(translator.t("msg.no_cpu_info"))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(translator.t("title.cpu_cores"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
            );
        f.render_widget(message, area);
        return;
    }
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45), // Charts & Overview
            Constraint::Percentage(55), // Detailed Cores Grid
        ])
        .split(area);
        
    let cpu_model = state.system_info.iter().find(|(k, _)| k == "CPU").map(|(_, v)| v.as_str()).unwrap_or("Unknown CPU");
    let core_details = state.system_info.iter().find(|(k, _)| k == "Cores").map(|(_, v)| v.as_str()).unwrap_or("Unknown");
    let vendor = state.system_info.iter().find(|(k, _)| k == "Vendor").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let family = state.system_info.iter().find(|(k, _)| k == "Family").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let l1_cache = state.system_info.iter().find(|(k, _)| k == "L1 Cache").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let l2_cache = state.system_info.iter().find(|(k, _)| k == "L2 Cache").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let l3_cache = state.system_info.iter().find(|(k, _)| k == "L3 Cache").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let bogomips = state.system_info.iter().find(|(k, _)| k == "BogoMIPS").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    let virt = state.system_info.iter().find(|(k, _)| k == "Virtualization").map(|(_, v)| v.as_str()).unwrap_or("N/A");
    
    let usage = &state.dynamic_data.global_usage;
    let numa_nodes = &state.dynamic_data.numa_nodes;
    let is_multi_numa = numa_nodes.len() > 1;
    let show_numa_panel = !numa_nodes.is_empty() && chunks[0].width >= 90;
    
    let top_chunks = if show_numa_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // Usage History Chart
                Constraint::Percentage(37), // CPU Overview
                Constraint::Percentage(28), // NUMA Architecture & Locality
            ])
            .split(chunks[0])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0])
    };
    
    let avg_freq = if !cores.is_empty() {
        cores.iter().map(|c| c.freq).sum::<u64>() as f64 / cores.len() as f64
    } else {
        0.0
    };
    
    let package_temp = state.dynamic_data.temperatures.cpu_temp;
    let max_core_temp = cores.iter().filter_map(|c| c.temp).fold(0.0f32, |a, b| a.max(b));
    let avg_core_temp = if !cores.is_empty() {
        let temps: Vec<f32> = cores.iter().filter_map(|c| c.temp).collect();
        if !temps.is_empty() {
            Some(temps.iter().sum::<f32>() / temps.len() as f32)
        } else {
            None
        }
    } else {
        None
    };

    let mut info_text = vec![
        Line::from(vec![
            Span::styled("Model: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(cpu_model, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Vendor: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(vendor, Style::default().fg(theme.text)),
            Span::raw(" | "),
            Span::styled("Family: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(family, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{}: ", translator.t("info.cores")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(core_details, Style::default().fg(theme.text)),
            Span::raw(" | "),
            Span::styled(format!("{}: ", translator.t("overview.load")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.1}%", usage.cpu), Style::default().fg(get_usage_color(usage.cpu))),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", translator.t("cpu.frequency")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.2} GHz (Avg)", avg_freq / 1000.0), Style::default().fg(theme.text)),
            Span::raw(" | "),
            Span::styled(format!("{} ", translator.t("cpu.bogomips")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(bogomips, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", translator.t("cpu.l1_cache")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(l1_cache, Style::default().fg(theme.text)),
            Span::raw(" | "),
            Span::styled(format!("{} ", translator.t("cpu.l2_cache")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(l2_cache, Style::default().fg(theme.text)),
            Span::raw(" | "),
            Span::styled(format!("{} ", translator.t("cpu.l3_cache")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(l3_cache, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", translator.t("cpu.virtualization")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(virt, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", translator.t("cpu.package")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(
                package_temp.map(|t| crate::utils::format_temp(t, state.temp_unit_fahrenheit)).unwrap_or_else(|| "N/A".to_string()),
                Style::default().fg(package_temp.map(get_usage_color).unwrap_or(theme.text_secondary))
            ),
            Span::raw(" | "),
            Span::styled(format!("{} ", translator.t("cpu.avg_core")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(
                avg_core_temp.map(|t| crate::utils::format_temp(t, state.temp_unit_fahrenheit)).unwrap_or_else(|| "N/A".to_string()),
                Style::default().fg(avg_core_temp.map(get_usage_color).unwrap_or(theme.text_secondary))
            ),
            Span::raw(" | "),
            Span::styled(format!("{} ", translator.t("cpu.max_core")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(crate::utils::format_temp(max_core_temp, state.temp_unit_fahrenheit), Style::default().fg(get_usage_color(max_core_temp))),
        ]),
        Line::from(vec![
             Span::styled(format!("{} ", translator.t("cpu.load_avg")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
             Span::styled(format!("{:.2} {:.2} {:.2}", usage.load_average.0, usage.load_average.1, usage.load_average.2), Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", translator.t("cpu.uptime")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(crate::utils::format_duration(usage.uptime), Style::default().fg(theme.text)),
        ]),
    ];

    if !numa_nodes.is_empty() && !show_numa_panel {
        let numa_summary = if numa_nodes.len() == 1 {
            let n = &numa_nodes[0];
            format!("1 Node ({} cores | RAM: {})", n.cpus.len(), crate::utils::format_size(n.mem_total_bytes))
        } else {
            format!("{} Nodes detected", numa_nodes.len())
        };

        info_text.push(Line::from(vec![
            Span::styled("NUMA: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(numa_summary, Style::default().fg(theme.text)),
        ]));
    }
    
    let info_paragraph = Paragraph::new(info_text)
        .block(Block::default()
            .title(format!(" {} ", translator.t("cpu.overview")))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
        );
    f.render_widget(info_paragraph, top_chunks[1]);

    let history_data: Vec<(f64, f64)> = state.dynamic_data.global_usage.cpu_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(translator.t("cpu.total_usage"))
            .marker(ratatui::symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.primary))
            .data(&history_data)
    ];
    
    let chart = Chart::new(datasets)
        .block(Block::default()
            .title(format!(" {} ", translator.t("cpu.usage_history")))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
        )
        .x_axis(Axis::default().bounds([0.0, 60.0]).style(Style::default().fg(theme.text_secondary)))
        .y_axis(Axis::default()
            .bounds([0.0, 100.0])
            .labels(vec![
                Span::styled("0%", Style::default().fg(theme.text_secondary)),
                Span::styled("100%", Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().fg(theme.text_secondary)));
    f.render_widget(chart, top_chunks[0]);

    if show_numa_panel {
        let mut numa_lines = Vec::new();
        for node in numa_nodes {
            let mem_pct = if node.mem_total_bytes > 0 {
                (node.mem_used_bytes as f64 / node.mem_total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let bar_len: usize = 10;
            let filled = ((mem_pct.clamp(0.0, 100.0) / 100.0) * bar_len as f64).round() as usize;
            let empty = bar_len.saturating_sub(filled);
            let bar = format!("▐{}{}▌", "█".repeat(filled), "░".repeat(empty));

            numa_lines.push(Line::from(vec![
                Span::styled(format!("Node {}: ", node.id), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} Cores", node.cpus.len()), Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({})", if node.cpu_list_str.is_empty() { format!("0-{}", node.cpus.len().saturating_sub(1)) } else { node.cpu_list_str.clone() }), Style::default().fg(theme.text_secondary)),
            ]));
            numa_lines.push(Line::from(vec![
                Span::styled("  RAM:  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(bar, Style::default().fg(get_usage_color(mem_pct as f32))),
                Span::styled(format!(" {:.1}%", mem_pct), Style::default().fg(get_usage_color(mem_pct as f32)).add_modifier(Modifier::BOLD)),
            ]));
            numa_lines.push(Line::from(vec![
                Span::styled("        ", Style::default()),
                Span::styled(format!("{} / {}", 
                    crate::utils::format_size(node.mem_used_bytes), 
                    crate::utils::format_size(node.mem_total_bytes)
                ), Style::default().fg(theme.text)),
            ]));
            if node.mem_cached_bytes > 0 || node.mem_free_bytes > 0 {
                numa_lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", translator.t("cpu.cache")), Style::default().fg(theme.text_secondary)),
                    Span::styled(format!("{} ", crate::utils::format_size(node.mem_cached_bytes)), Style::default().fg(theme.text)),
                    Span::styled(format!("| {} ", translator.t("cpu.free")), Style::default().fg(theme.text_secondary)),
                    Span::styled(crate::utils::format_size(node.mem_free_bytes), Style::default().fg(theme.text)),
                ]));
            }
            if let (Some(h), Some(m)) = (node.numa_hit, node.numa_miss) {
                let total = h + m;
                let hit_ratio = if total > 0 { (h as f64 / total as f64) * 100.0 } else { 100.0 };
                numa_lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", translator.t("cpu.hits")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:.1}%", hit_ratio), Style::default().fg(if hit_ratio > 95.0 { theme.success } else { theme.warning }).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" {}", translator.t("cpu.locality")), Style::default().fg(theme.text)),
                    Span::styled(if m == 0 { " (Local)".to_string() } else { format!(" ({}/{} hit/miss)", h, m) }, Style::default().fg(theme.text_secondary)),
                ]));
            }
            if numa_nodes.len() == 1 {
                numa_lines.push(Line::from(vec![
                    Span::styled("  Mode: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(translator.t("cpu.smp_mode"), Style::default().fg(theme.text_secondary)),
                ]));
            }
            if numa_nodes.len() > 1 {
                numa_lines.push(Line::raw(""));
            }
        }

        let numa_title = if numa_nodes.len() == 1 {
            format!(" {} ", translator.t("cpu.numa_locality"))
        } else {
            format!(" {} ", translator.t("cpu.numa_arch"))
        };

        let numa_block = Block::default()
            .title(numa_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));

        f.render_widget(Paragraph::new(numa_lines).block(numa_block), top_chunks[2]);
    }
    
    // --- Detailed Core Usage Grid with Multi-Core Scalability ---
    let inner_area = chunks[1];
    let num_cores = cores.len();
    let available_width = inner_area.width.saturating_sub(2);
    let available_height = inner_area.height.saturating_sub(2);

    // Compute optimal balanced grid
    let mut use_compact = num_cores > 24;
    let mut cores_per_row = 1u16;
    let mut rows_needed = 1u16;
    let mut row_height = 3u16;

    if !use_compact {
        let max_rows_by_height = (available_height / 3).max(1);
        let min_card_w: u16 = 20;

        let mut best_score = i64::MIN;
        let mut best_cfg = None;

        for r in 1..=max_rows_by_height.min(num_cores as u16) {
            let cols = (num_cores as u16).div_ceil(r);
            let col_w = available_width / cols.max(1);
            if col_w >= min_card_w {
                let row_h = (available_height / r).clamp(3, 5);
                let empty_slots = (r * cols).saturating_sub(num_cores as u16);
                let h_score = match row_h {
                    4 => 50,
                    5 => 45,
                    3 => 20,
                    _ => 0,
                };
                let w_score = if (24..=45).contains(&col_w) { 40 } else { 15 };
                let balance_score = 100 - (empty_slots as i64 * 35);
                let total_score = h_score + w_score + balance_score;
                if total_score > best_score {
                    best_score = total_score;
                    best_cfg = Some((r, cols, row_h));
                }
            }
        }

        if let Some((r, cols, rh)) = best_cfg {
            rows_needed = r;
            cores_per_row = cols;
            row_height = rh;
        } else {
            use_compact = true;
        }
    }

    if use_compact {
        let compact_col_width = 22;
        let cols = (available_width / compact_col_width).clamp(2, 8);
        rows_needed = (num_cores as u16).div_ceil(cols);
        cores_per_row = cols;
        row_height = 1;
    }

    let visible_rows = (available_height / row_height) as usize;
    let max_scroll = (rows_needed as usize).saturating_sub(visible_rows);
    let current_scroll = state.cpu_cores_scroll.min(max_scroll);

    let title = if rows_needed as usize > visible_rows {
        let start_row = current_scroll + 1;
        let end_row = (current_scroll + visible_rows).min(rows_needed as usize);
        format!(" {} ({} {}) [Rows {}-{} of {} | {}] ", translator.t("cpu.detailed_cores"), num_cores, translator.t("info.cores"), start_row, end_row, rows_needed, translator.t("cpu.scroll_hint"))
    } else {
        format!(" {} ({} {}) ", translator.t("cpu.detailed_cores"), num_cores, translator.t("info.cores"))
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
        
    let grid_area = block.inner(inner_area);
    f.render_widget(block, inner_area);

    if rows_needed == 0 || grid_area.height == 0 {
        return;
    }

    let rows_to_render = visible_rows.min((rows_needed as usize).saturating_sub(current_scroll));
    let row_constraints: Vec<Constraint> = (0..rows_to_render)
        .map(|_| Constraint::Length(row_height))
        .collect();

    let rows_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(grid_area);

    for (display_idx, row_area) in rows_layout.iter().enumerate() {
        let row_idx = current_scroll + display_idx;
        let start_core = row_idx * cores_per_row as usize;
        if start_core >= num_cores {
            break;
        }

        let cores_in_this_row = (num_cores - start_core).min(cores_per_row as usize);
        let core_constraints = vec![Constraint::Ratio(1, cores_per_row as u32); cores_per_row as usize];

        let cores_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(core_constraints)
            .split(*row_area);

        for (i, core_area) in cores_layout.iter().take(cores_in_this_row).enumerate() {
            let core_idx = start_core + i;
            let core = &cores[core_idx];
            let core_color = get_usage_color(core.usage);

            let core_label = if is_multi_numa {
                let numa_id = numa_nodes.iter()
                    .find(|n| n.cpus.contains(&core_idx))
                    .map(|n| n.id)
                    .unwrap_or(0);
                format!("N{}:C{:02}", numa_id, core_idx)
            } else {
                format!("C{:02}", core_idx)
            };

            if use_compact {
                let bar_len: usize = 6;
                let filled = ((core.usage.clamp(0.0, 100.0) / 100.0) * bar_len as f32).round() as usize;
                let empty = bar_len.saturating_sub(filled);
                let bar_str = format!("▐{}{}▌", "█".repeat(filled), "░".repeat(empty));

                let temp_str = core.temp
                    .map(|t| format!(" {}", crate::utils::format_temp_int(t, state.temp_unit_fahrenheit)))
                    .unwrap_or_default();

                let spans = if core_area.width >= 24 {
                    vec![
                        Span::styled(format!(" {} ", core_label), Style::default().bg(core_color).fg(theme.background).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {}", bar_str), Style::default().fg(core_color)),
                        Span::styled(format!(" {:>4.1}%", core.usage), Style::default().fg(core_color).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {:.1}G", core.freq as f64 / 1000.0), Style::default().fg(theme.text_secondary)),
                        Span::styled(temp_str, Style::default().fg(theme.text_secondary)),
                    ]
                } else if core_area.width >= 18 {
                    vec![
                        Span::styled(format!(" {} ", core_label), Style::default().bg(core_color).fg(theme.background).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {}", bar_str), Style::default().fg(core_color)),
                        Span::styled(format!(" {:>4.1}%", core.usage), Style::default().fg(core_color).add_modifier(Modifier::BOLD)),
                    ]
                } else {
                    vec![
                        Span::styled(format!(" {} ", core_label), Style::default().bg(core_color).fg(theme.background).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {:>4.0}%", core.usage), Style::default().fg(core_color).add_modifier(Modifier::BOLD)),
                    ]
                };

                f.render_widget(Paragraph::new(Line::from(spans)), *core_area);
            } else {
                let border_style = if core.usage > 80.0 {
                    Style::default().fg(theme.error)
                } else if core.usage > 50.0 {
                    Style::default().fg(theme.warning)
                } else {
                    Style::default().fg(theme.border)
                };

                let core_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style);

                let inner_core_area = core_block.inner(*core_area);
                f.render_widget(core_block, *core_area);

                let temp_str = core.temp
                    .map(|t| format!(" {}", crate::utils::format_temp_int(t, state.temp_unit_fahrenheit)))
                    .unwrap_or_default();

                if inner_core_area.height >= 2 {
                    // Line 1: Header (Badge, Freq, Temp)
                    let header_spans = vec![
                        Span::styled(format!(" {} ", core_label), Style::default().bg(core_color).fg(theme.background).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {:.2} GHz", core.freq as f64 / 1000.0), Style::default().fg(theme.text)),
                        Span::styled(temp_str, Style::default().fg(core.temp.map(get_usage_color).unwrap_or(theme.text_secondary))),
                    ];

                    // Line 2: Progress bar with percentage
                    let bar_width = (inner_core_area.width as usize).saturating_sub(9);
                    let filled = if bar_width > 0 {
                        ((core.usage.clamp(0.0, 100.0) / 100.0) * bar_width as f32).round() as usize
                    } else {
                        0
                    };
                    let empty = bar_width.saturating_sub(filled);
                    let bar_str = format!("▐{}{}▌", "█".repeat(filled), "░".repeat(empty));

                    let bar_spans = vec![
                        Span::styled(bar_str, Style::default().fg(core_color)),
                        Span::styled(format!(" {:>4.1}%", core.usage), Style::default().fg(core_color).add_modifier(Modifier::BOLD)),
                    ];

                    let mut lines = vec![
                        Line::from(header_spans),
                        Line::from(bar_spans),
                    ];

                    if inner_core_area.height >= 3 {
                        let load_tag = if core.usage < 15.0 {
                            translator.t("cpu.idle")
                        } else if core.usage < 65.0 {
                            translator.t("cpu.normal")
                        } else {
                            translator.t("cpu.heavy_load")
                        };
                        let sub_spans = vec![
                            Span::styled(format!("{}: {}", translator.t("label.load"), load_tag), Style::default().fg(theme.text_secondary)),
                        ];
                        lines.push(Line::from(sub_spans));
                    }

                    f.render_widget(Paragraph::new(lines), inner_core_area);
                } else {
                    // Single inner line (height == 1): render compact meter inside box
                    let bar_len: usize = 6;
                    let filled = ((core.usage.clamp(0.0, 100.0) / 100.0) * bar_len as f32).round() as usize;
                    let empty = bar_len.saturating_sub(filled);
                    let bar_str = format!("▐{}{}▌", "█".repeat(filled), "░".repeat(empty));

                    let line = Line::from(vec![
                        Span::styled(format!(" {} ", core_label), Style::default().bg(core_color).fg(theme.background).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {}", bar_str), Style::default().fg(core_color)),
                        Span::styled(format!(" {:>4.1}%", core.usage), Style::default().fg(core_color).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" {:.1}G", core.freq as f64 / 1000.0), Style::default().fg(theme.text)),
                        Span::styled(temp_str, Style::default().fg(theme.text_secondary)),
                    ]);

                    f.render_widget(Paragraph::new(line), inner_core_area);
                }
            }
        }
    }
}

fn render_disks_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let disks = &state.dynamic_data.disks;
    let headers: Vec<Cell> = vec![
        Cell::from(translator.t("disks.mount")),
        Cell::from(translator.t("disks.device")),
        Cell::from(translator.t("disks.fs")),
        Cell::from(translator.t("disks.total")),
        Cell::from(translator.t("disks.used")),
        Cell::from(translator.t("disks.free")),
        Cell::from(translator.t("disks.use_pct")),
        Cell::from(translator.t("disks.inodes")),
        Cell::from(translator.t("disks.ino_pct")),
        Cell::from(translator.t("disks.read_rate")),
        Cell::from(translator.t("disks.write_rate")),
        Cell::from(translator.t("disks.temp")),
        Cell::from(translator.t("disks.health")),
        Cell::from(translator.t("disks.cycles")),
        Cell::from(translator.t("disks.type")),
        Cell::from(translator.t("disks.mount_options")),
    ];
    
    let rows = disks.iter().map(|disk| {
        let usage_percent = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        
        let inodes_display = match (disk.inodes_used, disk.inodes_total) {
            (Some(used), Some(total)) => format!("{}/{}", format_count(used), format_count(total)),
            _ => "-".to_string(),
        };
        let inodes_pct_display = match (disk.inodes_used, disk.inodes_total) {
            (Some(used), Some(total)) if total > 0 => format!("{:.1}%", used as f64 / total as f64 * 100.0),
            _ => "-".to_string(),
        };
        let temp_display = disk.temp.map(|t| crate::utils::format_temp_int(t, state.temp_unit_fahrenheit)).unwrap_or_else(|| "-".to_string());
        let health_display = disk.health_pct.map(|h| format!("{}%", h)).unwrap_or_else(|| "-".to_string());
        let cycles_display = disk.power_cycles.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string());
        let type_display = match disk.is_ssd {
            Some(true) => if disk.device.to_lowercase().contains("nvme") { "NVMe" } else { "SSD" },
            Some(false) => "HDD",
            None => "-",
        };
        let mount_options_display = disk.mount_options.as_deref().unwrap_or("-");
        
        Row::new(vec![
            truncate_string(&disk.name, 15),
            truncate_string(&disk.device, 18),
            disk.fs.clone(),
            format_size(disk.total),
            format_size(disk.used),
            format_size(disk.free),
            format_percentage(usage_percent),
            inodes_display,
            inodes_pct_display,
            format_rate(disk.read_rate),
            format_rate(disk.write_rate),
            temp_display,
            health_display,
            cycles_display,
            type_display.to_string(),
            mount_options_display.to_string(),
        ]).style(Style::default().fg(
            if usage_percent > 90.0 { theme.error }
            else if usage_percent > 75.0 { theme.warning }
            else { theme.text }
        ))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Min(8),      // Mount
            Constraint::Length(14),  // Device
            Constraint::Length(6),   // FS
            Constraint::Length(9),   // Total
            Constraint::Length(9),   // Used
            Constraint::Length(9),   // Free
            Constraint::Length(7),   // Use%
            Constraint::Length(14),  // Inodes (U/T)
            Constraint::Length(7),   // Ino%
            Constraint::Length(10),  // Read Rate
            Constraint::Length(10),  // Write Rate
            Constraint::Length(6),   // Temp
            Constraint::Length(7),   // Health
            Constraint::Length(8),   // Cycles
            Constraint::Length(5),   // Type
            Constraint::Min(15),     // Mount Options
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title(format!(" {} ", translator.t("disks.title")))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    f.render_widget(table, area);
}

fn render_network_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    if is_safe_mode {
        let message = Paragraph::new(translator.t("msg.network_disabled_safe"))
            .style(Style::default().fg(theme.text_secondary))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(translator.t("title.network_interfaces"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.text_secondary))
            );
        f.render_widget(message, area);
        return;
    }
    
    let networks = &state.dynamic_data.networks;
    let iface_height = ((networks.len() as u16) + 4).clamp(5, 9);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(iface_height), // Interfaces
            Constraint::Min(6),               // Active Sockets & Connections
        ])
        .split(area);

    let headers: Vec<Cell> = vec![
        Cell::from(translator.t("network.iface")),
        Cell::from(translator.t("network.status")),
        Cell::from(translator.t("network.down_rate")),
        Cell::from(translator.t("network.up_rate")),
        Cell::from(translator.t("network.total_down")),
        Cell::from(translator.t("network.total_up")),
        Cell::from(translator.t("network.packets")),
    ];
    
    let rows = networks.iter().map(|net| {
        Row::new(vec![
            net.name.clone(),
            if net.is_up { "UP".to_string() } else { "DOWN".to_string() },
            format_rate(net.down_rate),
            format_rate(net.up_rate),
            format_size(net.total_down),
            format_size(net.total_up),
            format!("{}/{}", net.packets_rx, net.packets_tx),
        ]).style(Style::default().fg(
            if net.is_up { theme.success } else { theme.error }
        ))
    });
    
    let iface_table = Table::new(
        rows,
        [
            Constraint::Min(12),     // Interface
            Constraint::Length(8),   // Status
            Constraint::Length(12),  // Download/s
            Constraint::Length(12),  // Upload/s
            Constraint::Length(12),  // Total Down
            Constraint::Length(12),  // Total Up
            Constraint::Length(15),  // Packets
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title(format!(" {} ", translator.t("title.network_interfaces")))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    f.render_widget(iface_table, chunks[0]);

    // Active Sockets & Connections table
    let sockets = &state.dynamic_data.sockets;
    let socket_headers: Vec<Cell> = vec![
        Cell::from(translator.t("network.proto")),
        Cell::from(translator.t("network.local_addr")),
        Cell::from(translator.t("network.remote_addr")),
        Cell::from(translator.t("network.state")),
        Cell::from(translator.t("network.pid_prog")),
        Cell::from(translator.t("network.inode")),
    ];

    let total_sockets = sockets.len();
    let socket_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));

    let inner_area = socket_block.inner(chunks[1]);
    let visible_rows = inner_area.height.saturating_sub(1) as usize;

    let (visible_sockets, range_title) = if total_sockets == 0 {
        (&[][..], format!(" {} (0) ", translator.t("network.active_sockets")))
    } else {
        let max_scroll = total_sockets.saturating_sub(visible_rows);
        let scroll = state.network_socket_scroll.min(max_scroll);
        let end = (scroll + visible_rows).min(total_sockets);
        let slice = &sockets[scroll..end];
        let title = format!(
            " {} [{}-{}/{}] ",
            translator.t("network.active_sockets"),
            scroll + 1,
            end,
            total_sockets
        );
        (slice, title)
    };

    let socket_rows = visible_sockets.iter().map(|s| {
        let local = if s.local_port == 0 {
            if matches!(s.protocol, crate::types::SocketProtocol::Tcp6 | crate::types::SocketProtocol::Udp6) {
                format!("[{}]:*", s.local_addr)
            } else {
                format!("{}:*", s.local_addr)
            }
        } else if matches!(s.protocol, crate::types::SocketProtocol::Tcp6 | crate::types::SocketProtocol::Udp6) {
            format!("[{}]:{}", s.local_addr, s.local_port)
        } else {
            format!("{}:{}", s.local_addr, s.local_port)
        };

        let remote = if s.remote_port == 0 {
            if matches!(s.protocol, crate::types::SocketProtocol::Tcp6 | crate::types::SocketProtocol::Udp6) {
                format!("[{}]:*", s.remote_addr)
            } else {
                format!("{}:*", s.remote_addr)
            }
        } else if matches!(s.protocol, crate::types::SocketProtocol::Tcp6 | crate::types::SocketProtocol::Udp6) {
            format!("[{}]:{}", s.remote_addr, s.remote_port)
        } else {
            format!("{}:{}", s.remote_addr, s.remote_port)
        };

        let proc_display = match (s.pid, &s.process_name) {
            (Some(pid), Some(name)) => format!("{}/{}", pid, name),
            (Some(pid), None) => pid.to_string(),
            _ => "-".to_string(),
        };

        let state_style = match s.state.as_str() {
            "LISTEN" => Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            "ESTABLISHED" | "ESTAB" => Style::default().fg(theme.success),
            "TIME_WAIT" | "CLOSE_WAIT" | "SYN_SENT" | "SYN_RECV" => Style::default().fg(theme.warning),
            _ => Style::default().fg(theme.text_secondary),
        };

        Row::new(vec![
            Cell::from(s.protocol.as_str().to_string()).style(Style::default().fg(theme.accent)),
            Cell::from(truncate_string(&local, 30)),
            Cell::from(truncate_string(&remote, 30)),
            Cell::from(s.state.clone()).style(state_style),
            Cell::from(truncate_string(&proc_display, 24)).style(Style::default().fg(theme.highlight)),
            Cell::from(if s.inode > 0 { s.inode.to_string() } else { "-".to_string() }).style(Style::default().fg(theme.text_secondary)),
        ]).style(Style::default().fg(theme.text))
    });

    let socket_table = Table::new(
        socket_rows,
        [
            Constraint::Length(6),   // Proto
            Constraint::Min(22),     // Local Address
            Constraint::Min(22),     // Remote Address
            Constraint::Length(13),  // State
            Constraint::Length(24),  // PID/Program
            Constraint::Length(10),  // Inode
        ],
    )
    .header(
        Row::new(socket_headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
    )
    .block(
        socket_block
            .title(range_title)
            .title(
                ratatui::widgets::block::Title::from(format!(" [{}] ", translator.t("cpu.scroll_hint")))
                    .alignment(Alignment::Right),
            ),
    );

    f.render_widget(socket_table, chunks[1]);
}

fn render_containers_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    use ratatui::widgets::BorderType; 
    if let Some(err) = &state.dynamic_data.docker_error {
        let text = Paragraph::new(format!("Docker Error: {}", err))
             .style(Style::default().fg(theme.error))
             .alignment(Alignment::Center)
             .block(
                 Block::default()
                     .borders(Borders::ALL)
                     .border_type(BorderType::Rounded)
                     .style(Style::default().fg(theme.border))
                     .title(translator.t("msg.docker_unavailable"))
             );
        f.render_widget(text, area);
        return;
    }

    if state.dynamic_data.containers.is_empty() {
        let text = Paragraph::new(translator.t("msg.docker_disabled_or_none"))
             .style(Style::default().fg(theme.text_secondary))
             .alignment(Alignment::Center)
             .block(
                 Block::default()
                     .borders(Borders::ALL)
                     .border_type(BorderType::Rounded)
                     .style(Style::default().fg(theme.border))
                     .title(translator.t("title.containers"))
             );
        f.render_widget(text, area);
        return;
    }
    
    let containers = &state.dynamic_data.containers;
    
    let headers: Vec<Cell> = vec![
        Cell::from(translator.t("containers.id")),
        Cell::from(translator.t("header.name")),
        Cell::from(translator.t("containers.image")),
        Cell::from(translator.t("header.status")),
        Cell::from(translator.t("header.cpu")),
        Cell::from(translator.t("header.memory")),
        Cell::from(translator.t("containers.net_down")),
        Cell::from(translator.t("containers.net_up")),
        Cell::from(translator.t("containers.disk_r")),
        Cell::from(translator.t("containers.disk_w")),
        Cell::from(translator.t("containers.ports")),
    ];
    
    let rows = containers.iter().map(|c| {
        let status_color = if c.status.to_lowercase().contains("up") {
            theme.success
        } else if c.status.to_lowercase().contains("exit") {
            theme.error
        } else {
            theme.warning
        };
        
        Row::new(vec![
            c.id.clone(),
            truncate_string(&c.name, 20),
            truncate_string(&c.image, 25),
            c.status.clone(),
            c.cpu.clone(),
            c.mem.clone(),
            c.net_down.clone(),
            c.net_up.clone(),
            c.disk_r.clone(),
            c.disk_w.clone(),
            truncate_string(&c.ports, 20),
        ]).style(Style::default().fg(status_color))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),  // ID
            Constraint::Min(15),     // Name
            Constraint::Length(25),  // Image
            Constraint::Length(12),  // Status
            Constraint::Length(8),   // CPU
            Constraint::Length(10),  // Memory
            Constraint::Length(10),  // Net Down
            Constraint::Length(10),  // Net Up
            Constraint::Length(10),  // Disk Read
            Constraint::Length(10),  // Disk Write
            Constraint::Min(15),     // Ports
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ")
    .block(
        Block::default()
            .title(format!(" {} ", {
                let title_fmt = translator.t("containers.title_with_controls");
                if title_fmt.contains("{}") {
                    title_fmt.replacen("{}", &containers.len().to_string(), 1)
                } else {
                    format!("{} ({})", title_fmt, containers.len())
                }
            }))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    let mut container_state = state.container_table_state.clone();
    f.render_stateful_widget(table, area, &mut container_state);
}

fn render_gpu_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    if is_safe_mode {
        let message = Paragraph::new(translator.t("msg.gpu_disabled_safe"))
            .style(Style::default().fg(theme.text_secondary))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(translator.t("title.gpu"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.text_secondary))
            );
        f.render_widget(message, area);
        return;
    }
    
    let block = Block::default()
        .title(translator.t("title.gpu"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.border));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    match &state.dynamic_data.gpus {
        Ok(gpus) if gpus.is_empty() => {
            let message = Paragraph::new(translator.t("msg.no_gpus_found"))
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.warning));
            f.render_widget(message, inner_area);
        }
        Ok(gpus) => {
            render_gpu_details(f, gpus, inner_area, theme, state.temp_unit_fahrenheit, translator);
        }
        Err(e) => {
            let message = Paragraph::new(format!("GPU Error: {}", e))
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.error));
            f.render_widget(message, inner_area);
        }
    }
}

fn render_gpu_details(f: &mut Frame, gpus: &[crate::types::GpuInfo], area: Rect, theme: &crate::ui::colors::ColorScheme, fahrenheit: bool, translator: &Translator) {
    let num_gpus = gpus.len();
    if num_gpus == 0 {
        return;
    }
    
    let constraints: Vec<Constraint> = (0..num_gpus)
        .map(|_| Constraint::Ratio(1, num_gpus as u32))
        .collect();
    
    let gpu_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    
    for (i, gpu) in gpus.iter().enumerate() {
        if i >= gpu_layout.len() {
            continue;
        }
        
        render_single_gpu(f, gpu, gpu_layout[i], i, theme, fahrenheit, translator);
    }
}

fn render_single_gpu(f: &mut Frame, gpu: &crate::types::GpuInfo, area: Rect, index: usize, theme: &crate::ui::colors::ColorScheme, fahrenheit: bool, translator: &Translator) {
    let temp_str = crate::utils::format_temp_int(gpu.temperature as f32, fahrenheit);
    let active_str = translator.t("status.active");
    let title = if gpu.is_throttling {
        format!(" GPU {} - {} [{}: {}] ", index, temp_str, translator.t("overview.throttled"), gpu.throttle_reasons.as_deref().unwrap_or(&active_str))
    } else {
        format!(" GPU {} - {} ", index, temp_str)
    };

    let border_color = if gpu.is_throttling {
        theme.warning
    } else {
        theme.border
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Name
            Constraint::Length(1),  // Gauge
            Constraint::Percentage(30), // Utilization Chart
            Constraint::Percentage(30), // Memory Chart
            Constraint::Min(5),     // Details
        ])
        .split(inner_area);
    
    let name_line = Line::from(vec![
        Span::styled(format!("{} ", gpu.brand), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(&gpu.name, Style::default().fg(theme.text)),
    ]);
    f.render_widget(Paragraph::new(name_line), layout[0]);

    let util_color = get_usage_color(gpu.utilization as f32);
    let util_gauge = Gauge::default()
        .label(format!("{}: {}%", translator.t("gpu.utilization"), gpu.utilization))
        .gauge_style(Style::default().fg(util_color))
        .ratio(gpu.utilization as f64 / 100.0);
    f.render_widget(util_gauge, layout[1]);
    
    let data: Vec<(f64, f64)> = gpu.utilization_history
        .iter()
        .enumerate()
        .map(|(i, &u)| (i as f64, u as f64))
        .collect();
        
    let dataset = Dataset::default()
        .name(translator.t("gpu.utilization"))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(util_color))
        .data(&data);
        
    let chart = Chart::new(vec![dataset])
        .x_axis(Axis::default()
            .bounds([0.0, 60.0])
            .style(Style::default().fg(theme.text_secondary)))
        .y_axis(Axis::default()
            .bounds([0.0, 100.0])
            .labels(vec![
                Span::styled("0%", Style::default().fg(theme.text_secondary)),
                Span::styled("100%", Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().fg(theme.text_secondary)))
        .block(
             Block::default()
                .title(format!(" {} ", translator.t("gpu.util_history")))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
        );
    f.render_widget(chart, layout[2]);

    let mem_data: Vec<(f64, f64)> = gpu.memory_history
        .iter()
        .enumerate()
        .map(|(i, &u)| (i as f64, u as f64))
        .collect();
        
    let mem_dataset = Dataset::default()
        .name(translator.t("title.memory"))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(theme.accent))
        .data(&mem_data);
        
    let mem_chart = Chart::new(vec![mem_dataset])
        .x_axis(Axis::default()
            .bounds([0.0, 60.0])
            .style(Style::default().fg(theme.text_secondary)))
        .y_axis(Axis::default()
            .bounds([0.0, 100.0])
            .labels(vec![
                Span::styled("0%", Style::default().fg(theme.text_secondary)),
                Span::styled("100%", Style::default().fg(theme.text_secondary)),
            ])
            .style(Style::default().fg(theme.text_secondary)))
        .block(
             Block::default()
                .title(format!(" {} ", translator.t("gpu.mem_history")))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
        );
    f.render_widget(mem_chart, layout[3]);
    
    let mem_percent = if gpu.memory_total > 0 {
        (gpu.memory_used as f64 / gpu.memory_total as f64 * 100.0) as f32
    } else {
        0.0
    };
    
    let cur_pcie = match (gpu.pci_link_gen, gpu.pci_link_width) {
        (Some(gen), Some(width)) => Some(format!("Gen {} x{}", gen, width)),
        (Some(gen), None) => Some(format!("Gen {}", gen)),
        _ => None,
    };
    let max_pcie = match (gpu.pci_link_gen_max, gpu.pci_link_width_max) {
        (Some(gen), Some(width)) => Some(format!("Gen {} x{}", gen, width)),
        (Some(gen), None) => Some(format!("Gen {}", gen)),
        _ => None,
    };
    let pcie_str = match (cur_pcie, max_pcie) {
        (Some(cur), Some(max)) if cur != max => format!("{} (Max {})", cur, max),
        (Some(cur), Some(_)) => format!("{} (Max)", cur),
        (Some(cur), None) => cur,
        (None, Some(max)) => format!("Max {}", max),
        (None, None) => "N/A".to_string(),
    };

    let mem_util_str = gpu.memory_utilization
        .map(|u| format!("{}% (eng)", u))
        .unwrap_or_else(|| "N/A".to_string());

    let power_str = if let Some(limit) = gpu.power_limit {
        format!("{:.1} W / {:.1} W", gpu.power_usage as f64 / 1000.0, limit as f64 / 1000.0)
    } else {
        format!("{:.2} W", gpu.power_usage as f64 / 1000.0)
    };

    let throttle_cell = if gpu.is_throttling {
        Cell::from(Span::styled(
            gpu.throttle_reasons.as_deref().unwrap_or(&active_str),
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
        ))
    } else {
        Cell::from(Span::styled(
            gpu.throttle_reasons.as_deref().unwrap_or("None"),
            Style::default().fg(theme.success),
        ))
    };

    let fan_str = if let Some(fan) = gpu.fan_speed {
        if let Some(rpm) = gpu.fan_rpm {
            format!("{}% ({} RPM)", fan, rpm)
        } else {
            format!("{}%", fan)
        }
    } else if let Some(rpm) = gpu.fan_rpm {
        format!("{} RPM", rpm)
    } else {
        "N/A".to_string()
    };

    let mut table_rows = vec![
        Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.mem_usage"), Style::default().fg(theme.accent))),
            Cell::from(format!("{} / {} ({:.1}%)", format_size(gpu.memory_used), format_size(gpu.memory_total), mem_percent)),
            Cell::from(Span::styled(translator.t("gpu.core_clock"), Style::default().fg(theme.accent))),
            Cell::from(format!("{} MHz", gpu.graphics_clock)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.mem_engine"), Style::default().fg(theme.accent))),
            Cell::from(mem_util_str),
            Cell::from(Span::styled(translator.t("gpu.mem_clock"), Style::default().fg(theme.accent))),
            Cell::from(format!("{} MHz", gpu.memory_clock)),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.power_draw_lim"), Style::default().fg(theme.accent))),
            Cell::from(power_str),
            Cell::from(Span::styled(translator.t("gpu.pcie_link"), Style::default().fg(theme.accent))),
            Cell::from(pcie_str),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.throttling"), Style::default().fg(theme.accent))),
            throttle_cell,
            Cell::from(Span::styled(translator.t("gpu.fan_speed"), Style::default().fg(theme.accent))),
            Cell::from(fan_str),
        ]),
        Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.driver"), Style::default().fg(theme.accent))),
            Cell::from(gpu.driver_version.clone()),
            Cell::from(Span::styled(translator.t("gpu.brand"), Style::default().fg(theme.accent))),
            Cell::from(gpu.brand.clone()),
        ]),
    ];

    if let Some(temp) = gpu.memory_temperature {
        table_rows.push(Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.vram_temp"), Style::default().fg(theme.accent))),
            Cell::from(crate::utils::format_temp_int(temp as f32, fahrenheit)),
            Cell::from(Span::styled(translator.t("gpu.junction_temp"), Style::default().fg(theme.accent))),
            Cell::from(gpu.vram_temp.map(|t| crate::utils::format_temp_int(t as f32, fahrenheit)).unwrap_or_else(|| "N/A".to_string())),
        ]));
    } else if let Some(junc) = gpu.vram_temp {
        table_rows.push(Row::new(vec![
            Cell::from(Span::styled(translator.t("gpu.junction_temp"), Style::default().fg(theme.accent))),
            Cell::from(crate::utils::format_temp_int(junc as f32, fahrenheit)),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ]
    ).block(Block::default().borders(Borders::NONE));

    f.render_widget(table, layout[4]);
}

fn render_system_info_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // System Info Key-Value Table
            Constraint::Percentage(35), // Active User Sessions Table
            Constraint::Percentage(15), // Process & Reboot Summary
        ])
        .split(area);
    
    let rows = state.system_info.iter().map(|(key, value)| {
        Row::new(vec![key.clone(), value.clone()]).style(Style::default().fg(theme.text))
    });
    
    let table = Table::new(
        rows,
        [Constraint::Length(22), Constraint::Min(30)]
    )
    .block(
        Block::default()
            .title(translator.t("title.system_info"))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    )
    .column_spacing(2);
    
    f.render_widget(table, layout[0]);

    // Active User Sessions
    let user_rows: Vec<Row> = if state.user_sessions.is_empty() {
        vec![
            Row::new(vec![
                if state.user_sessions_loaded { translator.t("system.no_sessions") } else { translator.t("system.loading_sessions") },
                "—".to_string(),
                "—".to_string(),
                "—".to_string(),
            ]).style(Style::default().fg(theme.text_secondary))
        ]
    } else {
        state.user_sessions.iter().map(|s| {
            Row::new(vec![
                s.user.clone(),
                s.line.clone(),
                s.login_time.clone(),
                s.host.clone(),
            ]).style(Style::default().fg(theme.text))
        }).collect()
    };

    let user_headers: Vec<Cell> = vec![
        Cell::from(translator.t("system.user")),
        Cell::from(translator.t("system.line")),
        Cell::from(translator.t("system.login_time")),
        Cell::from(translator.t("system.host")),
    ];
    let user_table = Table::new(
        user_rows,
        [
            Constraint::Length(18),
            Constraint::Length(16),
            Constraint::Length(24),
            Constraint::Min(20),
        ]
    )
    .header(
        Row::new(user_headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title(format!(" {} ", {
                let title_fmt = translator.t("system.sessions_title");
                if title_fmt.contains("{}") {
                    title_fmt.replacen("{}", &state.user_sessions.len().to_string(), 1)
                } else {
                    format!("{} ({})", title_fmt, state.user_sessions.len())
                }
            }))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    )
    .column_spacing(2);

    f.render_widget(user_table, layout[1]);
    
    use crate::utils::count_process_states;
    let (running, sleeping, zombie, other) = count_process_states(&state.dynamic_data.processes);
    
    let reboot_badge = if state.dynamic_data.reboot_required {
        translator.t("system.reboot_yes")
    } else {
        translator.t("system.reboot_no")
    };

    let summary_fmt = translator.t("system.procs_summary");
    let stats_text = summary_fmt
        .replacen("{}", &running.to_string(), 1)
        .replacen("{}", &sleeping.to_string(), 1)
        .replacen("{}", &zombie.to_string(), 1)
        .replacen("{}", &other.to_string(), 1)
        .replacen("{}", &state.dynamic_data.processes.len().to_string(), 1)
        .replacen("{}", &reboot_badge, 1);
    
    let stats_style = if state.dynamic_data.reboot_required {
        Style::default().fg(theme.warning)
    } else {
        Style::default().fg(theme.text)
    };

    let stats = Paragraph::new(stats_text)
        .alignment(Alignment::Left)
        .style(stats_style)
        .block(
            Block::default()
                .title(translator.t("title.process_stats"))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(if state.dynamic_data.reboot_required {
                    Style::default().fg(theme.warning)
                } else {
                    Style::default().fg(theme.border)
                })
        );
    
    f.render_widget(stats, layout[2]);
}

fn render_footer(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let usage = &state.dynamic_data.global_usage;
    
    let mut alerts = Vec::new();
    
    if usage.cpu > 85.0 {
        alerts.push(translator.t("alert.high_cpu"));
    }
    
    let mem_percent = if usage.mem_total > 0 {
        (usage.mem_used as f64 / usage.mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    if mem_percent > 90.0 {
        alerts.push(translator.t("alert.critical_memory"));
    } else if mem_percent > 80.0 {
        alerts.push(translator.t("alert.high_memory"));
    }
    
    let full_disks = state.dynamic_data.disks.iter()
        .filter(|d| d.total > 0 && (d.used as f64 / d.total as f64) > 0.95)
        .count();
    
    if full_disks > 0 {
        alerts.push(translator.t("alert.disk_critical"));
    }
    
    let help_text = if state.paused {
        translator.t("help.paused")
    } else {
        match state.active_tab {
            0 => translator.t("help.dashboard"),
            1 => translator.t("help.process"),
            2 => translator.t("help.cpu_scroll"),
            8 => if state.services_subtab == 1 {
                translator.t("help.timers_scroll")
            } else {
                translator.t("help.services")
            },
            9 => translator.t("help.logs"),
            10 => translator.t("help.config"),
            11 => translator.t("help.containers"),
            12 => translator.t("help.sensors"),
            _ => translator.t("help.main"),
        }
    };
    
    let alert_text = if !alerts.is_empty() {
        format!("{}: {} | {}", translator.t("alert.title"), alerts.join(" | "), help_text)
    } else {
        help_text
    };
    
    let footer_style = if !alerts.is_empty() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if state.paused {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    
    let footer_msg = format!("{} | {}", alert_text, translator.t("help.settings_badge"));
    let footer = Paragraph::new(footer_msg)
        .style(footer_style)
        .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

fn render_services_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Subtab switcher
            Constraint::Min(0),    // Table view
        ])
        .split(area);

    let services_count = state.services.len();
    let timers_count = state.timers.len();

    let subtab_1 = translator.t("services.subtab_services").replacen("{}", &services_count.to_string(), 1);
    let subtab_2 = translator.t("services.subtab_timers").replacen("{}", &timers_count.to_string(), 1);
    let subtab_titles = vec![
        format!(" {} ", subtab_1),
        format!(" {} ", subtab_2),
    ];
    let tabs = Tabs::new(subtab_titles)
        .select(state.services_subtab)
        .style(Style::default().fg(theme.text_secondary))
        .highlight_style(Style::default().fg(theme.highlight).bg(theme.border).add_modifier(Modifier::BOLD))
        .divider("│")
        .block(
            Block::default()
                .title(format!(" {} ", translator.t("services.subtab_title")))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.primary))
        );
    f.render_widget(tabs, chunks[0]);

    if state.services_subtab == 0 {
        let services = &state.services;
        if services.is_empty() {
            let paragraph = Paragraph::new(translator.t("msg.no_services"))
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.text_secondary))
                .block(Block::default()
                    .title(translator.t("title.services"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.success)));
            f.render_widget(paragraph, chunks[1]);
            return;
        }
        
        let header_name = translator.t("header.name");
        let header_status = translator.t("header.status");
        let header_enabled = translator.t("header.enabled");
        
        let headers = vec![
            header_name.as_str(),
            header_status.as_str(),
            header_enabled.as_str(),
        ];
        
        let rows = services.iter().enumerate().map(|(i, s)| {
            let enabled = if s.enabled { "[+]" } else { "[-]" };
            let name_display = if state.has_sudo {
                s.name.clone()
            } else {
                format!("{} [RO]", s.name)
            };
            
            let style = if state.editing_service == Some(i) && state.has_sudo {
                Style::default().bg(theme.secondary).fg(theme.text)
            } else if !state.has_sudo {
                Style::default().fg(theme.text_secondary)
            } else {
                Style::default().fg(theme.text)
            };
            
            Row::new(vec![
                name_display,
                s.status.clone(),
                enabled.to_string(),
            ]).style(style)
        });
        
        let table = Table::new(
            rows,
            [
                Constraint::Length(28),
                Constraint::Length(16),
                Constraint::Length(12),
            ]
        )
        .header(
            Row::new(headers)
                .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        )
        .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title(if state.has_sudo {
                    translator.t("title.services")
                } else {
                    format!("{} {}", translator.t("title.services"), translator.t("modal.read_only"))
                })
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(if state.has_sudo {
                    Style::default().fg(theme.border)
                } else {
                    Style::default().fg(theme.text_secondary)
                })
        );
        
        let mut service_state = state.services_table_state.clone();
        f.render_stateful_widget(table, chunks[1], &mut service_state);
    } else {
        let timers = &state.timers;
        if timers.is_empty() {
            let msg = if state.timers_loaded {
                translator.t("timers.none")
            } else {
                translator.t("timers.loading")
            };
            let paragraph = Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.text_secondary))
                .block(Block::default()
                    .title(format!(" {} ", translator.t("timers.title")))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border)));
            f.render_widget(paragraph, chunks[1]);
            return;
        }

        let headers: Vec<Cell> = vec![
            Cell::from(translator.t("timers.unit")),
            Cell::from(translator.t("timers.next")),
            Cell::from(translator.t("timers.left")),
            Cell::from(translator.t("timers.last")),
            Cell::from(translator.t("timers.passed")),
            Cell::from(translator.t("timers.activates")),
        ];
        let rows = timers.iter().map(|t| {
            let style = if t.left.contains("min") || t.left.contains("s") {
                Style::default().fg(theme.highlight)
            } else {
                Style::default().fg(theme.text)
            };
            Row::new(vec![
                t.unit.clone(),
                t.next.clone(),
                t.left.clone(),
                t.last.clone(),
                t.passed.clone(),
                t.activates.clone(),
            ]).style(style)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(32),
                Constraint::Length(22),
                Constraint::Length(15),
                Constraint::Length(22),
                Constraint::Length(15),
                Constraint::Min(25),
            ]
        )
        .header(
            Row::new(headers)
                .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        )
        .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title(format!(" {} ({}) ", translator.t("timers.title"), timers.len()))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
        );

        let mut timers_state = state.timers_table_state.clone();
        f.render_stateful_widget(table, chunks[1], &mut timers_state);
    }
}

fn render_logs_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter & Boot
            Constraint::Min(0),    // Table
        ])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Filter
            Constraint::Percentage(50), // Boot
        ])
        .split(chunks[0]);

    let filter_text = if state.log_filter.is_empty() {
        translator.t("logs.press_to_filter")
    } else {
        translator.t("logs.filter_label").replacen("{}", &state.log_filter, 1)
    };

    let filter_style = if state.editing_filter {
        Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
    } else if !state.log_filter.is_empty() {
        Style::default().fg(theme.success)
    } else {
        Style::default().fg(theme.text_secondary)
    };

    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", translator.t("logs.filter_title")))
        .style(Style::default().fg(if state.editing_filter { theme.primary } else { theme.border }));

    let filter_widget = Paragraph::new(if state.editing_filter {
            format!("{}█", state.edit_buffer)
        } else {
            filter_text
        })
        .style(filter_style)
        .block(filter_block);

    f.render_widget(filter_widget, top_chunks[0]);

    let boot_text = if !state.boots.is_empty() {
        let current = state.boots.get(state.current_boot_idx)
            .map(|b| format!("{} ({})", b.id.get(..8).unwrap_or("?"), b.timestamp))
            .unwrap_or_else(|| "Unknown".to_string());
        format!(" < Boot {}/{} > : {} ", state.current_boot_idx + 1, state.boots.len(), current)
    } else {
        format!(" {} ", translator.t("logs.no_boot"))
    };

    let boot_widget = Paragraph::new(boot_text)
        .style(Style::default().fg(theme.text))
        .alignment(Alignment::Right)
        .block(
             Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", translator.t("logs.boot_title")))
                .style(Style::default().fg(theme.border))
        );
    
    f.render_widget(boot_widget, top_chunks[1]);

    let logs = &state.logs;
    
    if logs.is_empty() {
        let paragraph = Paragraph::new(translator.t("msg.no_logs"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_secondary))
            .block(Block::default()
                .title(translator.t("title.logs"))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.info)));
        f.render_widget(paragraph, chunks[1]);
        return;
    }
    
    let header_timestamp = translator.t("header.timestamp");
    let header_level = translator.t("header.level");
    let header_message = translator.t("header.message");
    
    let headers = vec![
        header_timestamp.as_str(),
        header_level.as_str(),
        header_message.as_str(),
    ];
    
    let rows = logs.iter().map(|l| {
        let level_color = match l.level.as_str() {
            "ERROR" => theme.error,
            "WARNING" => theme.warning,
            "INFO" => theme.success,
            "DEBUG" => theme.text_secondary,
            _ => theme.text,
        };
        
        Row::new(vec![
            l.timestamp.clone(),
            l.level.clone(),
            l.message.clone(),
        ]).style(Style::default().fg(level_color))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Min(40),
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .title(translator.t("title.logs"))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    let logs_state = state.logs_table_state.clone();
    f.render_stateful_widget(table, chunks[1], &mut logs_state.clone());
}

fn render_config_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let configs = &state.config_items;
    
    if configs.is_empty() {
        let paragraph = Paragraph::new(translator.t("msg.no_config"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text_secondary))
            .block(Block::default()
                .title(translator.t("title.config"))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)));
        f.render_widget(paragraph, area);
        return;
    }
    
    let header_key = translator.t("config.header_key");
    let header_value = translator.t("config.header_value");
    let header_desc = translator.t("config.header_desc");
    
    let headers = vec![
        header_key.as_str(),
        header_value.as_str(),
        header_desc.as_str(),
    ];
    
    let rows = configs.iter().enumerate().map(|(i, c)| {
        let style = if state.editing_config == Some(i) && state.has_sudo {
            Style::default().bg(theme.secondary).fg(theme.text)
        } else if !state.has_sudo {
            Style::default().fg(theme.text_secondary)
        } else {
            Style::default().fg(theme.text)
        };
        
        let display_val = if state.editing_config == Some(i) && state.has_sudo {
            format!("{}█", state.edit_buffer)
        } else {
            c.value.clone()
        };
        
        Row::new(vec![
            c.key.clone(),
            display_val,
            c.description.clone(),
        ]).style(style)
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(45),
            Constraint::Percentage(20),
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().bg(theme.border).fg(theme.highlight).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .title(if state.has_sudo {
                translator.t("title.config")
            } else {
                format!("{} {}", translator.t("title.config"), translator.t("modal.read_only"))
            })
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(if state.has_sudo {
                Style::default().fg(theme.border)
            } else {
                Style::default().fg(theme.text_secondary)
            })
    );
    
    let config_state = state.config_table_state.clone();
    f.render_stateful_widget(table, area, &mut config_state.clone());
}

fn render_memory_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // RAM & Swap Gauges
            Constraint::Percentage(50), // Details Table
        ])
        .split(area);

    let gauge_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // RAM
            Constraint::Percentage(50), // Swap
        ])
        .split(chunks[0]);

    let usage = &state.dynamic_data.global_usage;
    
    let mem_percent = if usage.mem_total > 0 {
        (usage.mem_used as f64 / usage.mem_total as f64) * 100.0
    } else { 0.0 };
    
    let mem_gauge = Gauge::default()
        .block(Block::default().title(translator.t("title.ram_usage")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(theme.border)))
        .gauge_style(Style::default().fg(get_usage_color(mem_percent as f32)))
        .percent(mem_percent as u16)
        .label(format!("{:.1}% ({} / {})", mem_percent, format_size(usage.mem_used), format_size(usage.mem_total)));
    f.render_widget(mem_gauge, gauge_chunks[0]);

    let swap_percent = if usage.swap_total > 0 {
         (usage.swap_used as f64 / usage.swap_total as f64) * 100.0
    } else { 0.0 }; 
    
    let swap_gauge = Gauge::default()
        .block(Block::default().title(translator.t("title.swap_usage")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(theme.border)))
        .gauge_style(Style::default().fg(theme.primary))
        .percent(swap_percent as u16)
        .label(format!("{:.1}% ({} / {})", swap_percent, format_size(usage.swap_used), format_size(usage.swap_total)));
    f.render_widget(swap_gauge, gauge_chunks[1]);

    let total_mem_str = format_size(usage.mem_used + (usage.mem_total - usage.mem_used));
    let used_mem_str = format_size(usage.mem_used);
    let cached_mem_str = format_size(usage.mem_cached);
    let free_mem_str = format_size(usage.mem_total.saturating_sub(usage.mem_used));

    let headers = vec![translator.t("memory.metric"), translator.t("memory.value")];

    let unknown_str = translator.t("memory.unknown");
    let (mem_type, mem_gen, mem_speed, mem_temp) = state.dynamic_data.global_usage.mem_details.clone()
        .unwrap_or_else(|| (unknown_str.clone(), unknown_str, "N/A".into(), "N/A".into()));

    let rows = vec![
        Row::new(vec![translator.t("memory.total_mem"), total_mem_str]), 
        Row::new(vec![translator.t("memory.used_mem"), used_mem_str]),
        Row::new(vec![translator.t("memory.cached_buffers"), cached_mem_str]),
        Row::new(vec![translator.t("memory.free_available"), free_mem_str]),
        Row::new(vec![translator.t("memory.type"), mem_type]),
        Row::new(vec![translator.t("memory.generation"), mem_gen]),
        Row::new(vec![translator.t("memory.speed"), mem_speed]),
        Row::new(vec![translator.t("memory.temperature"), mem_temp]),
    ];
    
    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)]
    ).header(Row::new(headers).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)))
     .block(Block::default().title(translator.t("title.details")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(theme.border)));
     
    f.render_widget(table, chunks[1]);
}

fn render_sensors_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let (bat_area, sensors_area) = if state.dynamic_data.battery.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Battery & Power Supply Card
                Constraint::Min(0),    // Hardware Sensors Table
            ])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    if let (Some(b_area), Some(bat)) = (bat_area, &state.dynamic_data.battery) {
        let is_charging = bat.status.eq_ignore_ascii_case("charging");
        let fill_len = (((bat.capacity as f64).clamp(0.0, 100.0) / 100.0) * 16.0) as usize;
        let empty_len = 16_usize.saturating_sub(fill_len);
        let bar = format!("[{}{}]", "█".repeat(fill_len), "░".repeat(empty_len));

        let health_str = if let Some(h) = bat.health_percent {
            let full_wh = bat.energy_full_wh.unwrap_or(0.0);
            let des_wh = bat.energy_design_wh.unwrap_or(0.0);
            if des_wh > 0.0 {
                format!("{:.1}% ({:.1} / {:.1} Wh)", h, full_wh, des_wh)
            } else {
                format!("{:.1}%", h)
            }
        } else {
            "—".to_string()
        };

        let power_str = bat.power_watts.map(|w| format!("{:.2} W", w)).unwrap_or_else(|| "—".into());
        let volt_str = bat.voltage_volts.map(|v| format!("{:.3} V", v)).unwrap_or_else(|| "—".into());
        let cycle_str = bat.cycle_count.map(|c| c.to_string()).unwrap_or_else(|| "—".into());
        let tech_str = bat.technology.as_deref().unwrap_or("—");
        let model_str = bat.model_name.as_deref().unwrap_or("—");
        let gov_str = bat.cpu_governor.as_deref().unwrap_or("—");
        let driver_str = bat.cpu_driver.as_deref().unwrap_or("");
        let gov_full = if driver_str.is_empty() {
            gov_str.to_string()
        } else {
            format!("{} ({})", gov_str, driver_str)
        };
        let ac_status = if bat.ac_online { "Connected (Online)" } else { "Disconnected (On Battery)" };

        let lines = vec![
            Line::from(vec![
                Span::styled(" Battery: ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}  {} {}%  Status: {}  Health: {}", bat.name, bar, bat.capacity, bat.status, health_str)),
            ]),
            Line::from(vec![
                Span::styled(" Power:   ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::raw(format!("Draw: {}  Voltage: {}  Cycles: {}  Tech: {}  Model: {}", power_str, volt_str, cycle_str, tech_str, model_str)),
            ]),
            Line::from(vec![
                Span::styled(" AC / CPU:", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::raw(translator.t("power.ac_cpu").replacen("{}", ac_status, 1).replacen("{}", &gov_full, 1)),
            ]),
        ];

        let bat_block = Block::default()
            .title(format!(" {} ", {
                let title_fmt = translator.t("power.title");
                if title_fmt.contains("{}") {
                    title_fmt.replacen("{}", &bat.name, 1)
                } else {
                    format!("{} ({})", title_fmt, bat.name)
                }
            }))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(if is_charging { theme.highlight } else { theme.border }));

        let bat_p = Paragraph::new(lines).block(bat_block);
        f.render_widget(bat_p, b_area);
    }

    let sensors = &state.dynamic_data.sensors;
    
    if sensors.is_empty() {
        let message = Paragraph::new(translator.t("msg.no_sensors"))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(translator.t("title.sensors"))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
            );
        f.render_widget(message, sensors_area);
        return;
    }

    let mut temp_sensors = Vec::new();
    let mut fan_sensors = Vec::new();
    let mut voltage_sensors = Vec::new();
    let mut power_sensors = Vec::new();
    let mut current_sensors = Vec::new();
    let mut other_sensors = Vec::new();

    for sensor in sensors.iter() {
        match sensor.sensor_type.as_str() {
            "temp" => temp_sensors.push(sensor),
            "fan" => fan_sensors.push(sensor),
            "in" => voltage_sensors.push(sensor),
            "power" => power_sensors.push(sensor),
            "curr" => current_sensors.push(sensor),
            _ => other_sensors.push(sensor),
        }
    }

    
    let mut rows: Vec<Row> = Vec::new();

    if !temp_sensors.is_empty() {
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.temperatures")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &temp_sensors {
            let ratio = if let Some(lim) = s.limit {
                if lim > 0.0 { s.value as f32 / lim } else { 0.0 }
            } else if let Some(crit) = s.critical {
                if crit > 0.0 { s.value as f32 / crit } else { 0.0 }
            } else {
                s.value as f32 / 85.0
            };
            let color = if ratio > 0.9 { theme.error } else if ratio > 0.75 { theme.warning } else { theme.success };
            let filled = ((ratio.min(1.0)) * 12.0) as usize;
            let empty = 12_usize.saturating_sub(filled);
            let bar = format!("▐{}{}▌", "█".repeat(filled), "░".repeat(empty));
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                crate::utils::format_temp(s.value as f32, state.temp_unit_fahrenheit),
                s.max.map(|v| crate::utils::format_temp(v, state.temp_unit_fahrenheit)).unwrap_or_else(|| "—".into()),
                s.limit.map(|v| crate::utils::format_temp(v, state.temp_unit_fahrenheit))
                    .or_else(|| s.critical.map(|v| format!("{} (crit)", crate::utils::format_temp(v, state.temp_unit_fahrenheit))))
                    .unwrap_or_else(|| "—".into()),
                bar,
            ]).style(Style::default().fg(color)));
        }
    }

    if !fan_sensors.is_empty() {
        rows.push(Row::new(vec![String::new(); 5]));
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.fan_speeds")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &fan_sensors {
            let color = if s.value > 0.0 { theme.success } else { theme.text_secondary };
            let status = if s.value > 0.0 { "* ACTIVE" } else { "o OFF" };
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                format!("{:.0} RPM", s.value),
                String::new(),
                String::new(),
                status.to_string(),
            ]).style(Style::default().fg(color)));
        }
    }

    if !voltage_sensors.is_empty() {
        rows.push(Row::new(vec![String::new(); 5]));
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.voltages")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &voltage_sensors {
            let color = theme.text;
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                format!("{:.3} V", s.value),
                String::new(),
                String::new(),
                String::new(),
            ]).style(Style::default().fg(color)));
        }
    }

    if !power_sensors.is_empty() {
        rows.push(Row::new(vec![String::new(); 5]));
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.power_section")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &power_sensors {
            let color = if s.value > 100.0 { theme.warning } else { theme.text };
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                format!("{:.2} W", s.value),
                String::new(),
                String::new(),
                String::new(),
            ]).style(Style::default().fg(color)));
        }
    }

    if !current_sensors.is_empty() {
        rows.push(Row::new(vec![String::new(); 5]));
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.current_section")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &current_sensors {
            let color = theme.text;
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                format!("{:.3} A", s.value),
                String::new(),
                String::new(),
                String::new(),
            ]).style(Style::default().fg(color)));
        }
    }

    if !other_sensors.is_empty() {
        rows.push(Row::new(vec![String::new(); 5])); // spacer
        rows.push(
            Row::new(vec![
                format!(" ═══ {} ═══", translator.t("sensors.other_section")),
                String::new(), String::new(), String::new(), String::new(),
            ]).style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
        );
        for s in &other_sensors {
            rows.push(Row::new(vec![
                format!("  {}", s.label),
                format!("{:.2} {}", s.value, s.unit),
                String::new(),
                String::new(),
                String::new(),
            ]).style(Style::default().fg(theme.text)));
        }
    }

    let count = sensors.len();
    let headers: Vec<Cell> = vec![
        Cell::from(translator.t("sensors.sensor")),
        Cell::from(translator.t("sensors.value")),
        Cell::from(translator.t("sensors.max_seen")),
        Cell::from(translator.t("sensors.limit")),
        Cell::from(translator.t("header.status")),
    ];
    
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(34),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(24),
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(
        Block::default()
            .title(format!(" {} ({}) ", translator.t("title.hardware_sensors"), count))
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
    );
    
    f.render_widget(table, sensors_area);
}

fn render_config_confirmation_modal(f: &mut Frame, key: &str, old_value: &str, new_value: &str, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    
    let popup_width = 80;
    let popup_height = 14;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };
    
    f.render_widget(ratatui::widgets::Clear, popup_area);
    
    let block = Block::default()
        .title(translator.t("title.confirm_config_change"))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning).add_modifier(Modifier::BOLD));
        
    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Key
            Constraint::Length(4), // Changes
            Constraint::Min(2),    // Prompt
        ])
        .margin(1)
        .split(inner_area);
        
    let key_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(format!("{}: ", translator.t("config.header_key")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(key, Style::default().fg(theme.text)),
        ])
    ]);
    f.render_widget(key_text, layout[0]);
    
    let change_text = Paragraph::new(vec![
        Line::from(Span::styled(translator.t("modal.config_current"), Style::default().fg(theme.text_secondary))),
        Line::from(Span::styled(format!("  {}", old_value), Style::default().fg(theme.text))),
        Line::from(""),
        Line::from(Span::styled(translator.t("modal.config_new"), Style::default().fg(theme.success).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("  {}", new_value), Style::default().fg(theme.highlight))),
    ]);
    f.render_widget(change_text, layout[1]);
    
    let prompt = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(translator.t("modal.config_confirm"), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(translator.t("modal.config_cancel"), Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
        ]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(prompt, layout[2]);
}

fn render_log_details_modal(f: &mut Frame, log: &crate::types::LogEntry, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    
    let popup_area = Rect {
        x: area.width / 10,
        y: area.height / 10,
        width: area.width * 8 / 10,
        height: area.height * 8 / 10,
    };
    
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let level_color = match log.level.as_str() {
        "ERROR" => theme.error,
        "WARNING" => theme.warning,
        "INFO" => theme.success,
        "DEBUG" => theme.text_secondary,
        _ => theme.text,
    };

    let level_str = format!("[{}]", log.level);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{}: ", translator.t("header.timestamp")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&log.timestamp, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled(format!("{}: ", translator.t("header.level")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&level_str, Style::default().fg(level_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("{}: ", translator.t("header.service")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(&log.service, Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{}:", translator.t("header.message")), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    for msg_line in log.message.lines() {
        lines.push(Line::from(Span::styled(msg_line, Style::default().fg(theme.text))));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(translator.t("modal.close_hint"), Style::default().fg(theme.text_secondary)),
    ]));

    let block = Block::default()
        .title(format!(" {}: {} ({}) ", translator.t("modal.log_details_title"), log.service, translator.t("modal.close_hint")))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.highlight));
        
    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.text))
        .wrap(ratatui::widgets::Wrap { trim: false });
        
    f.render_widget(paragraph, popup_area);
}

fn render_grub_update_modal(f: &mut Frame, state: &AppState, translator: &Translator, theme: &crate::ui::colors::ColorScheme) {
    let area = f.size();
    
    let popup_area = Rect {
        x: area.width / 10,
        y: area.height / 10,
        width: area.width * 8 / 10,
        height: area.height * 8 / 10,
    };
    
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(translator.t("modal.grub_warning"), Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(translator.t("modal.grub_desc1")),
        ]),
        Line::from(vec![
            Span::raw(translator.t("modal.grub_desc2")),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(translator.t("modal.grub_pending"), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    let mut has_changes = false;
    for item in &state.config_items {
        if item.value != item.original_value {
            has_changes = true;
            lines.push(Line::from(vec![
                Span::styled(format!("  - {}: ", item.key), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(format!("\"{}\"", item.original_value), Style::default().fg(theme.text_secondary)),
                Span::styled(" -> ", Style::default().fg(theme.warning)),
                Span::styled(format!("\"{}\"", item.value), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            ]));
        }
    }

    if !has_changes {
        lines.push(Line::from(vec![
            Span::styled(translator.t("modal.grub_no_changes"), Style::default().fg(theme.text_secondary)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("--------------------------------------------------------------------------------"),
    ]));
    lines.push(Line::from(""));
    
    if state.has_sudo {
        lines.push(Line::from(vec![
            Span::styled(translator.t("modal.grub_confirm_q"), Style::default().fg(theme.text)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(translator.t("modal.grub_yes"), Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
            Span::raw("   |   "),
            Span::styled(translator.t("modal.grub_cancel"), Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(translator.t("modal.grub_root_required"), Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(translator.t("modal.grub_close"), Style::default().fg(theme.text_secondary)),
        ]));
    }

    let block = Block::default()
        .title(format!(" {} ", translator.t("modal.grub_title")))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(theme.warning).add_modifier(Modifier::BOLD));
        
    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.text))
        .wrap(ratatui::widgets::Wrap { trim: false });
        
    f.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tab_at_column() {
        let translator = crate::language::Translator::new(crate::language::Language::English);
        // Col 0 is outside (starts at 1)
        assert_eq!(get_tab_at_column(0, &translator), None);
        // Col 1 is first char of "1:Dashboard"
        assert_eq!(get_tab_at_column(1, &translator), Some(0));
        assert_eq!(get_tab_at_column(5, &translator), Some(0));
        // Large column beyond tab bar
        assert_eq!(get_tab_at_column(500, &translator), None);
    }

    #[test]
    fn test_localized_ui_keys_all_languages() {
        use crate::language::Language;
        let test_keys = [
            "cpu.overview", "cpu.usage_history", "cpu.total_usage", "cpu.numa_locality",
            "cpu.numa_arch", "cpu.detailed_cores", "disks.title", "disks.mount",
            "disks.device", "network.iface", "network.active_sockets",
            "containers.title_with_controls", "gpu.utilization", "gpu.mem_history",
            "system.sessions_title", "services.subtab_services", "timers.title",
            "logs.filter_title", "config.header_key", "memory.total_mem",
            "sensors.temperatures", "modal.grub_warning", "modal.log_details_title",
        ];

        for lang in [
            Language::English,
            Language::Turkish,
            Language::French,
            Language::German,
            Language::Spanish,
            Language::Italian,
            Language::Russian,
        ] {
            let translator = crate::language::Translator::new(lang);
            for key in &test_keys {
                let text = translator.t(key);
                assert!(!text.is_empty(), "Language {:?} returned empty text for key {}", lang, key);
                assert_ne!(text, *key, "Language {:?} missing translation for key {}", lang, key);
            }
        }
    }
}