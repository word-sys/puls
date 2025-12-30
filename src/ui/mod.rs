pub mod widgets;
pub mod colors;
pub mod layouts;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Sparkline, Table, Tabs},
};

use crate::types::AppState;
use crate::utils::{format_size, format_rate, format_percentage, format_frequency, get_usage_color, truncate_string, get_system_health, get_top_memory_consumers, get_cpu_efficiency, estimate_memory_availability};
use crate::language::Translator;

pub use layouts::*;

pub fn render_ui(f: &mut Frame, state: &mut AppState, is_safe_mode: bool, translator: &Translator) {
    let main_layout = create_main_layout(f.size());
    
    render_tab_bar(f, state, main_layout.tab_area, is_safe_mode, translator);
    
    render_summary_bar(f, state, main_layout.summary_area, translator);
    
    match state.active_tab {
        0 => render_dashboard_tab(f, state, main_layout.content_area, translator),
        1 => render_process_detail_tab(f, state, main_layout.content_area, translator),
        2 => render_cpu_cores_tab(f, state, main_layout.content_area, translator),
        3 => render_disks_tab(f, state, main_layout.content_area, translator),
        4 => render_network_tab(f, state, main_layout.content_area, is_safe_mode, translator),
        5 => render_gpu_tab(f, state, main_layout.content_area, is_safe_mode, translator),
        6 => render_system_info_tab(f, state, main_layout.content_area, translator),
        7 => render_services_tab(f, state, main_layout.content_area, translator),
        8 => render_logs_tab(f, state, main_layout.content_area, translator),
        9 => render_config_tab(f, state, main_layout.content_area, translator),
        _ => {}
    }
    
    render_footer(f, state, main_layout.footer_area, translator);
}

fn render_tab_bar(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, translator: &Translator) {
    let tab_keys = vec![
        "tab.dashboard", "tab.process", "tab.cpu", "tab.disks", "tab.network", "tab.gpu", "tab.system", "tab.services", "tab.logs", "tab.config"
    ];
    let tab_titles: Vec<Line> = tab_keys
    .iter()
    .enumerate()
    .map(|(i, &key)| {
        let title = translator.t(key);
        let style = if is_safe_mode && (i == 4 || i == 5 || i == 7 || i == 8 || i == 9) {
            Style::default().fg(Color::DarkGray)
        } else if i == state.active_tab {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        Line::from(Span::styled(title, style))
    })
    .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default()
            .title(translator.t("title.puls"))
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .select(state.active_tab)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, area);
}

fn render_summary_bar(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
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
    
    render_cpu_gauge(f, usage.cpu, usage.load_average, layout[0], translator);
    
    render_memory_gauge(f, usage.mem_used, usage.mem_total, layout[1], translator);
    
    render_gpu_gauge(f, usage.gpu_util, layout[2], translator);
    
    render_network_summary(f, usage, layout[3], translator);
    
    render_disk_summary(f, usage, layout[4], translator);
}

fn render_cpu_gauge(f: &mut Frame, cpu_percent: f32, load_avg: (f64, f64, f64), area: Rect, translator: &Translator) {
    let color = get_usage_color(cpu_percent);
    let label = format!("{:.1}% | Load: {:.1}", cpu_percent, load_avg.0);
    let gauge = Gauge::default()
        .block(Block::default()
            .title(translator.t("title.cpu"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .gauge_style(Style::default().fg(color))
        .percent(cpu_percent.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

fn render_memory_gauge(f: &mut Frame, mem_used: u64, mem_total: u64, area: Rect, translator: &Translator) {
    let mem_percent = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let color = get_usage_color(mem_percent as f32);
    
    // Show memory pressure level
    let pressure = match mem_percent {
        x if x >= 90.0 => "health.critical",
        x if x >= 80.0 => "health.high",
        x if x >= 60.0 => "health.moderate",
        _ => "health.healthy",
    };
    
    let label = format!("{} ({}: {}%)", format_size(mem_used), translator.t(pressure), mem_percent as u16);
    
    let gauge = Gauge::default()
        .block(Block::default()
            .title(translator.t("title.memory"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .gauge_style(Style::default().fg(color))
        .percent(mem_percent.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

fn render_gpu_gauge(f: &mut Frame, gpu_util: Option<u32>, area: Rect, translator: &Translator) {
    let block = Block::default()
        .title(translator.t("title.gpu"))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
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
            .block(block);
        f.render_widget(paragraph, area);
    }
}

fn render_network_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect, translator: &Translator) {
    let block = Block::default()
        .title(translator.t("title.network"))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner_area);
    
    let net_text = format!("▼{} ▲{}", format_rate(usage.net_down), format_rate(usage.net_up));
    let net_paragraph = Paragraph::new(net_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(net_paragraph, layout[0]);
    
    if !usage.net_down_history.is_empty() || !usage.net_up_history.is_empty() {
        let combined_data: Vec<u64> = usage.net_down_history
            .iter()
            .zip(usage.net_up_history.iter())
            .map(|(&down, &up)| down.max(up))
            .collect();
        
        if !combined_data.is_empty() {
            let sparkline = Sparkline::default()
                .data(&combined_data)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, layout[1]);
        }
    }
}

fn render_disk_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect, translator: &Translator) {
    let block = Block::default()
        .title(translator.t("title.disk"))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner_area);
    
    let disk_text = format!("▼{} ▲{}", format_rate(usage.disk_read), format_rate(usage.disk_write));
    let disk_paragraph = Paragraph::new(disk_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::LightRed));
    f.render_widget(disk_paragraph, layout[0]);
    
    if !usage.disk_read_history.is_empty() || !usage.disk_write_history.is_empty() {
        let combined_data: Vec<u64> = usage.disk_read_history
            .iter()
            .zip(usage.disk_write_history.iter())
            .map(|(&read, &write)| read.max(write))
            .collect();
        
        if !combined_data.is_empty() {
            let sparkline = Sparkline::default()
                .data(&combined_data)
                .style(Style::default().fg(Color::LightRed));
            f.render_widget(sparkline, layout[1]);
        }
    }
}

fn render_dashboard_tab(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Percentage(57), Constraint::Percentage(40)])
        .split(area);
    
    render_system_status(f, state, layout[0], translator);
    
    render_process_table(f, state, layout[1], translator);
    
    render_container_table(f, state, layout[2], translator);
}

fn render_system_status(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let usage = &state.dynamic_data.global_usage;
    let system_info = &state.system_info;
    
    let cpu_cores = system_info.iter()
        .find(|(k, _)| k == "Cores")
        .and_then(|(_, v)| v.split_whitespace().next()?.parse::<usize>().ok())
        .unwrap_or(1);
    
    let (status_str, load_per_core) = get_system_health(
        usage.load_average.0,
        cpu_cores,
        usage.mem_used,
        usage.mem_total,
    );
    
    let mem_percent = if usage.mem_total > 0 {
        (usage.mem_used as f64 / usage.mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let cpu_efficiency = get_cpu_efficiency(usage.cpu, usage.load_average.0);
    let (mem_available, availability_level) = estimate_memory_availability(usage.mem_used, usage.mem_total);
    
    let status_text = format!(
        "Status {} | CPU: {:.0}% (Eff: {}) | Load: {:.2}/core | Memory: {:.0}% ({}: {}) | Up: {} | Tasks: {}",
        status_str,
        usage.cpu,
        cpu_efficiency,
        load_per_core.parse::<f64>().unwrap_or(0.0),
        mem_percent,
        availability_level,
        format_size(mem_available),
        crate::utils::format_uptime(usage.uptime),
        state.dynamic_data.processes.len()
    );
    
    let status_paragraph = Paragraph::new(status_text)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title(translator.t("title.system_overview"))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
        );
    
    f.render_widget(status_paragraph, area);
}

fn render_process_table(f: &mut Frame, state: &mut AppState, area: Rect, translator: &Translator) {
    let processes = &state.dynamic_data.processes;
    let header_pid = translator.t("header.pid");
    let header_name = translator.t("header.name");
    let header_user = translator.t("header.user");
    let header_cpu = translator.t("header.cpu");
    let header_memory = translator.t("header.memory");
    let header_disk_read = translator.t("header.disk_read");
    let header_disk_write = translator.t("header.disk_write");
    
    let rows = processes.iter().map(|p| {
        Row::new(vec![
            p.pid.clone(),
            truncate_string(&p.name, 20),
            truncate_string(&p.user, 12),
            p.cpu_display.clone(),
            p.mem_display.clone(),
            p.disk_read.clone(),
            p.disk_write.clone(),
        ])
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),   // PID
            Constraint::Min(15),     // Name
            Constraint::Length(12),  // User
            Constraint::Length(8),   // CPU
            Constraint::Length(10),  // Memory
            Constraint::Length(12),  // Read/s
            Constraint::Length(12),  // Write/s
        ]
    )
    .header(
        Row::new(vec![header_pid, header_name, header_user, header_cpu, header_memory, header_disk_read, header_disk_write])
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(
        Block::default()
            .title("Processes (↑↓ navigate, Enter details, s sort, f filter)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");
    
    f.render_stateful_widget(table, area, &mut state.process_table_state);
}

fn render_container_table(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let containers = &state.dynamic_data.containers;
    
    if containers.is_empty() {
        let message = if state.system_info.iter().any(|(k, v)| k == "Mode" && v.contains("Safe")) {
            translator.t("msg.container_disabled")
        } else {
            translator.t("msg.no_containers")
        };
        
        let paragraph = Paragraph::new(message)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(translator.t("title.containers"))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );
        f.render_widget(paragraph, area);
        return;
    }
    
    let h_pid = translator.t("header.pid");
    let h_name = translator.t("header.name");
    let h_status = translator.t("status.active");
    let h_cpu = translator.t("header.cpu");
    let h_mem = translator.t("header.memory");
    let h_disk_r = translator.t("header.disk_read");
    let h_disk_w = translator.t("header.disk_write");
    
    let headers = vec![
        h_pid.as_str(),
        h_name.as_str(),
        h_status.as_str(),
        h_cpu.as_str(),
        h_mem.as_str(),
        "Net ↓/s",
        "Net ↑/s",
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
        ])
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
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title(translator.t("title.containers"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    );
    
    f.render_widget(table, area);
}

fn render_process_detail_tab(f: &mut Frame, state: &AppState, area: Rect, _translator: &Translator) {
    let block = Block::default()
        .title("Process Details")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    if let Some(ref process) = state.dynamic_data.detailed_process {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner_area);
        
        let info_lines = vec![
            Line::from(vec![
                Span::styled("PID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&process.pid)
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&process.name)
            ]),
            Line::from(vec![
                Span::styled("User: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&process.user)
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&process.status)
            ]),
            Line::from(vec![
                Span::styled("Parent PID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(process.parent.as_deref().unwrap_or("N/A"))
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&process.start_time)
            ]),
            Line::from(vec![
                Span::styled("CPU Usage: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:.2}%", process.cpu_usage))
            ]),
            Line::from(vec![
                Span::styled("Memory (RSS): ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(format_size(process.memory_rss))
            ]),
            Line::from(vec![
                Span::styled("Memory (VMS): ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(format_size(process.memory_vms))
            ]),
            Line::from(vec![
                Span::styled("Threads: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(process.threads.to_string())
            ]),
        ];
        
        let final_info_lines: Vec<_> = if let Some(ref cwd) = process.cwd {
            info_lines.into_iter().chain(std::iter::once(
                Line::from(vec![
                    Span::styled("Working Dir: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(cwd)
                ])
            )).collect::<Vec<_>>()
        } else {
            info_lines
        };
        let info_paragraph = Paragraph::new(final_info_lines)
            .block(
                Block::default()
                    .title("Process Information")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            )
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(info_paragraph, layout[0]);
        
        let mut cmd_env_lines = vec![
            Line::from(Span::styled("Command:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::raw(&process.command)),
            Line::from(""),
            Line::from(Span::styled("Environment Variables:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
        ];
        
        for (i, env) in process.environ.iter().enumerate() {
            if i >= 20 {
                cmd_env_lines.push(Line::from(Span::raw("... (truncated)")));
                break;
            }
            cmd_env_lines.push(Line::from(Span::raw(env)));
        }
        
        let cmd_env_paragraph = Paragraph::new(cmd_env_lines)
            .block(
                Block::default()
                    .title("Command & Environment")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            )
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(cmd_env_paragraph, layout[1]);
        
    } else {
        let message = Paragraph::new("Select a process from the Dashboard tab (↑↓ to navigate, Enter to select)")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message, inner_area);
    }
}

fn render_cpu_cores_tab(f: &mut Frame, state: &AppState, area: Rect, _translator: &Translator) {
    let cores = &state.dynamic_data.cores;
    
    if cores.is_empty() {
        let message = Paragraph::new("No CPU core information available")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("CPU Cores")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );
        f.render_widget(message, area);
        return;
    }
    
    let block = Block::default()
        .title(format!("CPU Cores ({} total)", cores.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let cores_per_row = (inner_area.width / 25).max(1) as usize;
    let rows_needed = (cores.len() + cores_per_row - 1) / cores_per_row;
    
    if rows_needed == 0 {
        return;
    }
    
    let row_constraints: Vec<Constraint> = (0..rows_needed)
        .map(|_| Constraint::Length(2))
        .collect();
    
    let rows_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .margin(1)
        .split(inner_area);
    
    for (row_idx, row_area) in rows_layout.iter().enumerate() {
        let start_core = row_idx * cores_per_row;
        let end_core = (start_core + cores_per_row).min(cores.len());
        
        if start_core >= cores.len() {
            break;
        }
        
        let cores_in_row = end_core - start_core;
        let core_constraints: Vec<Constraint> = (0..cores_in_row)
            .map(|_| Constraint::Percentage((100 / cores_in_row) as u16))
            .collect();
        
        let cores_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(core_constraints)
            .split(*row_area);
        
        for (core_idx, core_area) in cores_layout.iter().enumerate() {
            let actual_core_idx = start_core + core_idx;
            if actual_core_idx >= cores.len() {
                break;
            }
            
            let core = &cores[actual_core_idx];
            let color = get_usage_color(core.usage);
            let freq_display = format_frequency(core.freq);
            
            let gauge = Gauge::default()
                .label(format!("C{} {} {:.1}%", actual_core_idx, freq_display, core.usage))
                .gauge_style(Style::default().fg(color))
                .ratio((core.usage / 100.0) as f64);
            
            f.render_widget(gauge, *core_area);
        }
    }
}

fn render_disks_tab(f: &mut Frame, state: &AppState, area: Rect, _translator: &Translator) {
    let disks = &state.dynamic_data.disks;
    let headers = ["Mount Point", "Device", "FS", "Total", "Used", "Free", "Usage %"];
    
    let rows = disks.iter().map(|disk| {
        let usage_percent = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        
        Row::new(vec![
            truncate_string(&disk.name, 20),
            truncate_string(&disk.device, 15),
            disk.fs.clone(),
            format_size(disk.total),
            format_size(disk.used),
            format_size(disk.free),
            format_percentage(usage_percent),
        ]).style(Style::default().fg(
            if usage_percent > 90.0 { Color::Red }
            else if usage_percent > 75.0 { Color::Yellow }
            else { Color::White }
        ))
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Min(15),     // Mount Point
            Constraint::Length(15),  // Device
            Constraint::Length(8),   // FS
            Constraint::Length(10),  // Total
            Constraint::Length(10),  // Used
            Constraint::Length(10),  // Free
            Constraint::Length(10),  // Usage %
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title("Disk Usage")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    );
    
    f.render_widget(table, area);
}

fn render_network_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, _translator: &Translator) {
    if is_safe_mode {
        let message = Paragraph::new("Network monitoring is disabled in safe mode")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("Network Interfaces")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
            );
        f.render_widget(message, area);
        return;
    }
    
    let networks = &state.dynamic_data.networks;
    let headers = ["Interface", "Status", "Download/s", "Upload/s", "Total Down", "Total Up", "Packets Rx/Tx"];
    
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
            if net.is_up { Color::Green } else { Color::Red }
        ))
    });
    
    let table = Table::new(
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
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .block(
        Block::default()
            .title("Network Interfaces")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    );
    
    f.render_widget(table, area);
}

fn render_gpu_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool, _translator: &Translator) {
    if is_safe_mode {
        let message = Paragraph::new("GPU monitoring is disabled in safe mode")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("GPU Information")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
            );
        f.render_widget(message, area);
        return;
    }
    
    let block = Block::default()
        .title("GPU Information")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    match &state.dynamic_data.gpus {
        Ok(gpus) if gpus.is_empty() => {
            let message = Paragraph::new("No supported GPUs found")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(message, inner_area);
        }
        Ok(gpus) => {
            render_gpu_details(f, gpus, inner_area);
        }
        Err(e) => {
            let message = Paragraph::new(format!("GPU Error: {}", e))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red));
            f.render_widget(message, inner_area);
        }
    }
}

fn render_gpu_details(f: &mut Frame, gpus: &[crate::types::GpuInfo], area: Rect) {
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
        
        render_single_gpu(f, gpu, gpu_layout[i], i);
    }
}

fn render_single_gpu(f: &mut Frame, gpu: &crate::types::GpuInfo, area: Rect, index: usize) {
    let title = format!(
        "GPU {} - {} ({}) - {}°C",
        index,
        truncate_string(&gpu.name, 25),
        gpu.brand,
        gpu.temperature
    );
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(inner_area);
    
    let util_color = get_usage_color(gpu.utilization as f32);
    let util_gauge = Gauge::default()
        .label(format!("Utilization: {}%", gpu.utilization))
        .gauge_style(Style::default().fg(util_color))
        .ratio(gpu.utilization as f64 / 100.0);
    f.render_widget(util_gauge, layout[0]);
    
    let mem_percent = if gpu.memory_total > 0 {
        (gpu.memory_used as f64 / gpu.memory_total as f64 * 100.0) as f32
    } else {
        0.0
    };
    
    let details = vec![
        Line::from(vec![
            Span::styled("Memory: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} / {} ({:.1}%)",
                format_size(gpu.memory_used),
                format_size(gpu.memory_total),
                mem_percent
            ))
        ]),
        Line::from(vec![
            Span::styled("Power: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.2} W", gpu.power_usage as f64 / 1000.0))
        ]),
        Line::from(vec![
            Span::styled("Graphics Clock: ", Style::default().fg(Color::Yellow)),
            Span::raw(format_frequency(gpu.graphics_clock as u64))
        ]),
        Line::from(vec![
            Span::styled("Memory Clock: ", Style::default().fg(Color::Yellow)),
            Span::raw(format_frequency(gpu.memory_clock as u64))
        ]),
    ];
    
    let final_details: Vec<_> = if let Some(fan_speed) = gpu.fan_speed {
        details.into_iter().chain(std::iter::once(
            Line::from(vec![
                Span::styled("Fan Speed: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}%", fan_speed))
            ])
        )).collect::<Vec<_>>()
    } else {
        details
    };
    let details_paragraph = Paragraph::new(final_details);
    f.render_widget(details_paragraph, layout[1]);
}

fn render_system_info_tab(f: &mut Frame, state: &AppState, area: Rect, _translator: &Translator) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);
    
    let rows = state.system_info.iter().map(|(key, value)| {
        Row::new(vec![key.clone(), value.clone()])
    });
    
    let table = Table::new(
        rows,
        [Constraint::Length(20), Constraint::Min(30)]
    )
    .block(
        Block::default()
            .title("System Information")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    )
    .column_spacing(2);
    
    f.render_widget(table, layout[0]);
    
    use crate::utils::count_process_states;
    let (running, sleeping, zombie, other) = count_process_states(&state.dynamic_data.processes);
    
    let stats_text = format!(
        "Process Summary: {} Running | {} Sleeping | {} Zombie | {} Other | Total: {}",
        running, sleeping, zombie, other,
        state.dynamic_data.processes.len()
    );
    
    let stats = Paragraph::new(stats_text)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title("Process Statistics")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
        );
    
    f.render_widget(stats, layout[1]);
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
        translator.t("help.main")
    };
    
    let alert_text = if !alerts.is_empty() {
        format!("{}: {} | {}", translator.t("alert.title"), alerts.join(" | "), help_text)
    } else {
        help_text
    };
    
    let footer_style = if !alerts.is_empty() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if state.paused {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    
    let footer = Paragraph::new(alert_text)
        .style(footer_style)
        .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}

fn render_services_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let services = &state.services;
    
    if services.is_empty() {
        let paragraph = Paragraph::new("No services available")
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(translator.t("title.services"))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)));
        f.render_widget(paragraph, area);
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
        let enabled = if s.enabled { "✓" } else { "✗" };
        let name_display = if state.has_sudo {
            s.name.clone()
        } else {
            format!("{} [RO]", s.name)
        };
        
        let style = if state.editing_service == Some(i) && state.has_sudo {
            Style::default().bg(Color::DarkGray)
        } else if !state.has_sudo {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
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
            Constraint::Length(25),
            Constraint::Length(15),
            Constraint::Length(10),
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .title(if state.has_sudo {
                translator.t("title.services")
            } else {
                format!("{} (Read-Only)", translator.t("title.services"))
            })
            .borders(Borders::ALL)
            .border_style(if state.has_sudo {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            })
    );
    
    let mut service_state = state.services_table_state.clone();
    f.render_stateful_widget(table, area, &mut service_state.clone());
}

fn render_logs_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let logs = &state.logs;
    
    if logs.is_empty() {
        let paragraph = Paragraph::new("No logs available")
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(translator.t("title.logs"))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)));
        f.render_widget(paragraph, area);
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
            "ERROR" => Color::Red,
            "WARNING" => Color::Yellow,
            "INFO" => Color::Green,
            "DEBUG" => Color::Gray,
            _ => Color::White,
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
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .title(translator.t("title.logs"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
    );
    
    let mut logs_state = state.logs_table_state.clone();
    f.render_stateful_widget(table, area, &mut logs_state.clone());
}

fn render_config_tab(f: &mut Frame, state: &AppState, area: Rect, translator: &Translator) {
    let configs = &state.config_items;
    
    if configs.is_empty() {
        let paragraph = Paragraph::new("No configuration items available")
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(translator.t("title.config"))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)));
        f.render_widget(paragraph, area);
        return;
    }
    
    let header_key = translator.t("config.grub_timeout");
    let header_value = translator.t("info.load");
    let header_desc = translator.t("header.message");
    
    let headers = vec![
        header_key.as_str(),
        header_value.as_str(),
        header_desc.as_str(),
    ];
    
    let rows = configs.iter().enumerate().map(|(i, c)| {
        let style = if state.editing_config == Some(i) && state.has_sudo {
            Style::default().bg(Color::DarkGray)
        } else if !state.has_sudo {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        
        Row::new(vec![
            c.key.clone(),
            c.value.clone(),
            c.description.clone(),
        ]).style(style)
    });
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(25),
            Constraint::Length(20),
            Constraint::Min(35),
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    )
    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .block(
        Block::default()
            .title(if state.has_sudo {
                translator.t("title.config")
            } else {
                format!("{} (Read-Only)", translator.t("title.config"))
            })
            .borders(Borders::ALL)
            .border_style(if state.has_sudo {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::DarkGray)
            })
    );
    
    let mut config_state = state.config_table_state.clone();
    f.render_stateful_widget(table, area, &mut config_state.clone());
}