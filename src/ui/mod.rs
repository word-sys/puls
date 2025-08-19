pub mod widgets;
pub mod colors;
pub mod layouts;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Sparkline, Table, Tabs},
};

use crate::types::AppState;
use crate::utils::{format_size, format_rate, format_percentage, get_usage_color, truncate_string};

pub use layouts::*;

pub fn render_ui(f: &mut Frame, state: &mut AppState, is_safe_mode: bool) {
    let main_layout = create_main_layout(f.size());
    
    render_tab_bar(f, state, main_layout.tab_area, is_safe_mode);
    
    render_summary_bar(f, state, main_layout.summary_area);
    
    match state.active_tab {
        0 => render_dashboard_tab(f, state, main_layout.content_area),
        1 => render_process_detail_tab(f, state, main_layout.content_area),
        2 => render_cpu_cores_tab(f, state, main_layout.content_area),
        3 => render_disks_tab(f, state, main_layout.content_area),
        4 => render_network_tab(f, state, main_layout.content_area, is_safe_mode),
        5 => render_gpu_tab(f, state, main_layout.content_area, is_safe_mode),
        6 => render_system_info_tab(f, state, main_layout.content_area),
        _ => {}
    }
    
    render_footer(f, state, main_layout.footer_area);
}

fn render_tab_bar(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool) {
    let tab_titles: Vec<Line> = [
        "1:Dashboard", "2:Process", "3:CPU", "4:Disks", "5:Network", "6:GPU", "7:System"
    ]
    .iter()
    .enumerate()
    .map(|(i, &title)| {
        let style = if is_safe_mode && (i == 4 || i == 5) {
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
            .title("PULS - System Monitor")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .select(state.active_tab)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, area);
}

fn render_summary_bar(f: &mut Frame, state: &AppState, area: Rect) {
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
    
    render_cpu_gauge(f, usage.cpu, layout[0]);
    
    render_memory_gauge(f, usage.mem_used, usage.mem_total, layout[1]);
    
    render_gpu_gauge(f, usage.gpu_util, layout[2]);
    
    render_network_summary(f, usage, layout[3]);
    
    render_disk_summary(f, usage, layout[4]);
}

fn render_cpu_gauge(f: &mut Frame, cpu_percent: f32, area: Rect) {
    let color = get_usage_color(cpu_percent);
    let gauge = Gauge::default()
        .block(Block::default()
            .title("CPU Usage")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .gauge_style(Style::default().fg(color).bg(Color::Black))
        .percent(cpu_percent.clamp(0.0, 100.0) as u16)
        .label(format!("{:.1}%", cpu_percent));
    f.render_widget(gauge, area);
}

fn render_memory_gauge(f: &mut Frame, mem_used: u64, mem_total: u64, area: Rect) {
    let mem_percent = if mem_total > 0 {
        (mem_used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };
    
    let color = get_usage_color(mem_percent as f32);
    let label = format!("{} / {}", format_size(mem_used), format_size(mem_total));
    
    let gauge = Gauge::default()
        .block(Block::default()
            .title("Memory Usage")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)))
        .gauge_style(Style::default().fg(color).bg(Color::Black))
        .percent(mem_percent.clamp(0.0, 100.0) as u16)
        .label(label);
    f.render_widget(gauge, area);
}

fn render_gpu_gauge(f: &mut Frame, gpu_util: Option<u32>, area: Rect) {
    let block = Block::default()
        .title("GPU Usage")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    if let Some(gpu_percent) = gpu_util {
        let color = get_usage_color(gpu_percent as f32);
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(color).bg(Color::Black))
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

fn render_network_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect) {
    let block = Block::default()
        .title("Network I/O")
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

fn render_disk_summary(f: &mut Frame, usage: &crate::types::GlobalUsage, area: Rect) {
    let block = Block::default()
        .title("Disk I/O")
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

fn render_dashboard_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    
    render_process_table(f, state, layout[0]);
    
    render_container_table(f, state, layout[1]);
}

fn render_process_table(f: &mut Frame, state: &mut AppState, area: Rect) {
    let processes = &state.dynamic_data.processes;
    let headers = ["PID", "Name", "User", "CPU %", "Memory", "Read/s", "Write/s"];
    
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
            Constraint::Length(10),  // Read/s
            Constraint::Length(10),  // Write/s
        ]
    )
    .header(
        Row::new(headers)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .bottom_margin(1)
    )
    .block(
        Block::default()
            .title("Processes (↑↓ to select, Enter for details)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    )
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");
    
    f.render_stateful_widget(table, area, &mut state.process_table_state);
}

fn render_container_table(f: &mut Frame, state: &AppState, area: Rect) {
    let containers = &state.dynamic_data.containers;
    
    if containers.is_empty() {
        let message = if state.system_info.iter().any(|(k, v)| k == "Mode" && v.contains("Safe")) {
            "Container monitoring disabled in safe mode"
        } else {
            "No containers found (Docker not running?)"
        };
        
        let paragraph = Paragraph::new(message)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("Containers")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );
        f.render_widget(paragraph, area);
        return;
    }
    
    let headers = ["ID", "Name", "Status", "CPU %", "Memory", "Net ↓/s", "Net ↑/s", "Disk R/s", "Disk W/s"];
    
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
            .title("Containers")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
    );
    
    f.render_widget(table, area);
}

fn render_process_detail_tab(f: &mut Frame, state: &AppState, area: Rect) {
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

fn render_cpu_cores_tab(f: &mut Frame, state: &AppState, area: Rect) {
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
            let freq_mhz = core.freq as f64 / 1_000_000.0;
            
            let gauge = Gauge::default()
                .label(format!("C{} {:.0}MHz {:.1}%", actual_core_idx, freq_mhz, core.usage))
                .gauge_style(Style::default().fg(color))
                .ratio((core.usage / 100.0) as f64);
            
            f.render_widget(gauge, *core_area);
        }
    }
}

fn render_disks_tab(f: &mut Frame, state: &AppState, area: Rect) {
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

fn render_network_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool) {
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

fn render_gpu_tab(f: &mut Frame, state: &AppState, area: Rect, is_safe_mode: bool) {
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
            Span::raw(format!("{} W", gpu.power_usage / 1000))
        ]),
        Line::from(vec![
            Span::styled("Graphics Clock: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} MHz", gpu.graphics_clock))
        ]),
        Line::from(vec![
            Span::styled("Memory Clock: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} MHz", gpu.memory_clock))
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

fn render_system_info_tab(f: &mut Frame, state: &AppState, area: Rect) {
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
    
    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, state: &AppState, area: Rect) {
    let help_text = if state.paused {
        "PAUSED - Press 'p' to resume | Quit: q | Tabs: 1-7 | Navigate: ↑↓ | Details: Enter"
    } else {
        "Quit: q | Tabs: 1-7, Tab/Shift+Tab | Navigate: ↑↓ | Details: Enter | Pause: p"
    };
    
    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(if state.paused { Color::Red } else { Color::DarkGray }))
        .alignment(Alignment::Center);
    
    f.render_widget(footer, area);
}