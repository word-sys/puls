use std::{
    collections::HashMap,
    io::{self, stdout},
    process,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use bollard::{container::StatsOptions, Docker};
use chrono::prelude::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::{future, stream::StreamExt};
use nvml_wrapper::Nvml;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, TableState, Tabs},
};
use sysinfo::{CpuExt, DiskExt, DiskUsage, NetworkExt, NetworksExt, Pid, PidExt, ProcessExt, System, SystemExt};
use tokio::runtime::Runtime;
use tokio::time::{Instant, timeout};
use users::{Users, UsersCache};

#[derive(Clone, Default)]
struct NetworkStats { rx: u64, tx: u64 }
#[derive(Clone, Default)]
struct ContainerIoStats { net_rx: u64, net_tx: u64, disk_r: u64, disk_w: u64 }
#[derive(Clone, Debug)]
struct ProcessInfo { pid: String, name: String, cpu: String, mem: String, disk_read: String, disk_write: String }
#[derive(Clone, Debug)]
struct ContainerInfo { id: String, name: String, status: String, cpu: String, mem: String, net_down: String, net_up: String, disk_r: String, disk_w: String }
#[derive(Clone, Debug, Default)]
struct GpuInfo { name: String, brand: String, utilization: u32, memory_used: u64, memory_total: u64, temperature: u32, power_usage: u32, graphics_clock: u32, memory_clock: u32 }
#[derive(Clone, Debug, Default)]
struct DetailedProcessInfo { pid: String, name: String, user: String, status: String, cpu_usage: f32, memory_rss: u64, memory_vms: u64, command: String, start_time: String, parent: Option<String>, environ: Vec<String> }
#[derive(Clone, Debug, Default)]
struct CoreInfo { usage: f32, freq: u64 }
#[derive(Clone, Debug, Default)]
struct DetailedDiskInfo { name: String, fs: String, total: u64, free: u64 }
#[derive(Clone, Debug, Default)]
struct DetailedNetInfo { name: String, down_rate: u64, up_rate: u64, total_down: u64, total_up: u64 }

#[derive(Clone, Default)]
struct GlobalUsage {
    cpu: f32,
    mem_used: u64,
    mem_total: u64,
    gpu_util: u32,
    net_down: u64,
    net_up: u64,
    disk_read: u64,
    disk_write: u64,
}

#[derive(Clone)]
struct DynamicData {
    processes: Vec<ProcessInfo>,
    detailed_process: Option<DetailedProcessInfo>,
    cores: Vec<CoreInfo>,
    disks: Vec<DetailedDiskInfo>,
    networks: Vec<DetailedNetInfo>,
    containers: Vec<ContainerInfo>,
    gpus: Result<Vec<GpuInfo>, String>,
    global_usage: GlobalUsage,
}

impl Default for DynamicData {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            detailed_process: None,
            cores: Vec::new(),
            disks: Vec::new(),
            networks: Vec::new(),
            containers: Vec::new(),
            gpus: Ok(Vec::new()),
            global_usage: GlobalUsage::default(),
        }
    }
}

#[derive(Clone, Default)]
struct AppState {
    active_tab: usize,
    process_table_state: TableState,
    selected_pid: Option<Pid>,
    system_info: Vec<(String, String)>,
    dynamic_data: DynamicData,
}

fn main() -> io::Result<()> {
    let rt = Runtime::new()?;
    let _guard = rt.enter();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_state = Arc::new(Mutex::new(AppState::default()));

    let mut sys = System::new_all();
    sys.refresh_all();

    {
        let mut state = app_state.lock().unwrap();
        state.system_info = vec![
            ("OS".into(), sys.long_os_version().unwrap_or_default()),
            ("Kernel".into(), sys.kernel_version().unwrap_or_default()),
            ("Hostname".into(), sys.host_name().unwrap_or_default()),
            ("CPU".into(), sys.cpus().get(0).map_or("N/A".into(), |c| c.brand().to_string())),
            ("Cores".into(), format!("{} Physical / {} Logical", sys.physical_core_count().unwrap_or(0), sys.cpus().len())),
            ("Total Memory".into(), format_size(sys.total_memory())),
        ];
    }

    let app_state_clone = app_state.clone();
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let mut sys = System::new_all();
            let docker = Docker::connect_with_local_defaults().ok();
            let nvml = Nvml::init();

            let mut prev_disk_usage: HashMap<Pid, DiskUsage> = HashMap::new();
            let mut prev_net_usage: HashMap<String, NetworkStats> = HashMap::new();
            let mut prev_container_stats: HashMap<String, ContainerIoStats> = HashMap::new();
            let mut last_update = Instant::now();

            loop {
                let selected_pid = app_state_clone.lock().unwrap().selected_pid;

                let new_data = update_dynamic_data(
                    &mut sys, &docker, &nvml,
                    &mut prev_disk_usage, &mut prev_net_usage, &mut prev_container_stats,
                    selected_pid, &mut last_update
                ).await;

                let mut state = app_state_clone.lock().unwrap();
                state.dynamic_data = new_data;

                if state.process_table_state.selected().is_none() && !state.dynamic_data.processes.is_empty() {
                    state.process_table_state.select(Some(0));
                }

                thread::sleep(Duration::from_secs(1));
            }
        });
    });

    loop {
        {
            let mut current_state = app_state.lock().unwrap();
            terminal.draw(|f| ui(f, &mut current_state))?;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let mut current_state = app_state.lock().unwrap();
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Tab => current_state.active_tab = (current_state.active_tab + 1) % 7,
                    KeyCode::BackTab => current_state.active_tab = (current_state.active_tab + 6) % 7,
                    KeyCode::Down if current_state.active_tab == 0 => {
                        let processes = &current_state.dynamic_data.processes;
                        if !processes.is_empty() {
                            let i = match current_state.process_table_state.selected() {
                                Some(i) => if i >= processes.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            current_state.process_table_state.select(Some(i));
                        }
                    }
                    KeyCode::Up if current_state.active_tab == 0 => {
                        let processes = &current_state.dynamic_data.processes;
                        if !processes.is_empty() {
                            let i = match current_state.process_table_state.selected() {
                                Some(i) => if i == 0 { processes.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            current_state.process_table_state.select(Some(i));
                        }
                    }
                    KeyCode::Enter if current_state.active_tab == 0 => {
                        if let Some(selected_index) = current_state.process_table_state.selected() {
                            if let Some(process) = current_state.dynamic_data.processes.get(selected_index) {
                                if let Ok(pid_val) = process.pid.parse::<usize>() {
                                    current_state.selected_pid = Some(Pid::from(pid_val));
                                    current_state.active_tab = 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn update_dynamic_data(
    sys: &mut System,
    docker: &Option<Docker>,
    nvml: &Result<Nvml, nvml_wrapper::error::NvmlError>,
    prev_disk_usage: &mut HashMap<Pid, DiskUsage>,
    prev_net_usage: &mut HashMap<String, NetworkStats>,
    prev_container_stats: &mut HashMap<String, ContainerIoStats>,
    selected_pid: Option<Pid>,
    last_update: &mut Instant
) -> DynamicData {
    let now = Instant::now();
    let elapsed_secs = now.duration_since(*last_update).as_secs_f64().max(1.0);
    *last_update = now;

    sys.refresh_all();

    let self_pid = process::id();
    let user_cache = UsersCache::new();

    let mut total_disk_read = 0;
    let mut total_disk_write = 0;

    let mut current_disk_usage = HashMap::new();
    let mut processes = sys.processes().iter()
        .filter(|(pid, _)| pid.as_u32() != self_pid)
        .map(|(pid, process)| {
            let disk_usage = process.disk_usage();
            let (read_rate, write_rate) = if let Some(prev) = prev_disk_usage.get(pid) {
                let read_bytes = (disk_usage.total_read_bytes.saturating_sub(prev.total_read_bytes) as f64 / elapsed_secs) as u64;
                let written_bytes = (disk_usage.total_written_bytes.saturating_sub(prev.total_written_bytes) as f64 / elapsed_secs) as u64;
                (read_bytes, written_bytes)
            } else { (0, 0) };
            total_disk_read += read_rate;
            total_disk_write += write_rate;
            current_disk_usage.insert(*pid, disk_usage);
            ProcessInfo {
                pid: pid.to_string(),
                name: process.name().to_string(),
                cpu: format!("{:.2}", process.cpu_usage()),
                mem: format_size(process.memory()),
                disk_read: format_rate(read_rate),
                disk_write: format_rate(write_rate),
            }
        })
        .collect::<Vec<_>>();
    processes.sort_by(|a, b| b.cpu.parse::<f32>().unwrap_or(0.0).partial_cmp(&a.cpu.parse::<f32>().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    *prev_disk_usage = current_disk_usage;

    let detailed_process = if let Some(pid) = selected_pid {
        sys.process(pid).map(|process| DetailedProcessInfo {
            pid: process.pid().to_string(), name: process.name().to_string(),
            user: process.user_id().and_then(|uid| user_cache.get_user_by_uid(**uid)).map_or("N/A".to_string(), |u| u.name().to_string_lossy().into_owned()),
            status: process.status().to_string(), cpu_usage: process.cpu_usage(),
            memory_rss: process.memory(), memory_vms: process.virtual_memory(),
            command: process.cmd().join(" "),
            start_time: if let chrono::LocalResult::Single(dt) = Utc.timestamp_opt(process.start_time() as i64, 0) {
                dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
            } else { "Invalid time".to_string() },
            parent: process.parent().map(|p| p.to_string()),
            environ: process.environ().to_vec(),
        })
    } else { None };

    let cores = sys.cpus().iter().map(|c| CoreInfo { usage: c.cpu_usage(), freq: c.frequency() }).collect();
    let disks = sys.disks().iter().map(|d| DetailedDiskInfo { name: d.mount_point().to_string_lossy().into_owned(), fs: String::from_utf8_lossy(d.file_system()).into_owned(), total: d.total_space(), free: d.available_space() }).collect();

    let mut total_net_down = 0;
    let mut total_net_up = 0;
    let mut current_net_usage = HashMap::new();
    let networks = sys.networks().iter().map(|(iface, data)| {
        let (down_rate, up_rate) = if let Some(prev) = prev_net_usage.get(iface) {
            let rx = (data.total_received().saturating_sub(prev.rx) as f64 / elapsed_secs) as u64;
            let tx = (data.total_transmitted().saturating_sub(prev.tx) as f64 / elapsed_secs) as u64;
            (rx, tx)
        } else { (0, 0) };
        total_net_down += down_rate;
        total_net_up += up_rate;
        current_net_usage.insert(iface.clone(), NetworkStats { rx: data.total_received(), tx: data.total_transmitted() });
        DetailedNetInfo { name: iface.clone(), down_rate, up_rate, total_down: data.total_received(), total_up: data.total_transmitted() }
    }).collect();
    *prev_net_usage = current_net_usage;

    let (containers, current_container_stats) = if let Some(docker) = docker {
        if docker.ping().await.is_ok() {
            if let Ok(summaries) = docker.list_containers::<String>(None).await {
                let stats_futures = summaries.iter().filter_map(|s| s.id.as_ref()).map(|id| {
                    let docker_clone = docker.clone();
                    let id_clone = id.clone();
                    async move {
                        let options = StatsOptions { stream: false, ..Default::default() };
                        let mut stats_stream = docker_clone.stats(&id_clone, Some(options));
                        let fut = stats_stream.next();
                        (id_clone, timeout(Duration::from_millis(500), fut).await)
                    }
                });

                let stats_results = future::join_all(stats_futures).await;
                let mut stats_map = stats_results.into_iter().filter_map(|(id, stats_result)| {
                    match stats_result {
                        Ok(Some(Ok(stats))) => Some((id, stats)),
                        _ => None,
                    }
                }).collect::<HashMap<_, _>>();

                let mut container_info_list = Vec::new();
                let mut current_container_stats_map = HashMap::new();

                for summary in summaries {
                    let id_full = summary.id.clone().unwrap_or_default();
                    let id = id_full.get(..12).unwrap_or("N/A").to_string();
                    let name = summary.names.as_ref().and_then(|n| n.first()).map_or("N/A", |s| s.strip_prefix('/').unwrap_or(s)).to_string();
                    let status = summary.status.as_deref().unwrap_or("N/A").to_string();

                    let (cpu, mem, net_down, net_up, disk_r, disk_w) = if let Some(stats) = stats_map.remove(&id_full) {
                        let prev_stats = prev_container_stats.get(&id_full).cloned().unwrap_or_default();
                        let mut current_stats = ContainerIoStats::default();

                        let cpu_delta = stats.cpu_stats.cpu_usage.total_usage.saturating_sub(stats.precpu_stats.cpu_usage.total_usage) as f64;
                        let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0).saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0)) as f64;
                        let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
                        let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 { format!("{:.2}%", (cpu_delta / system_delta) * num_cpus * 100.0) } else { "0.00%".to_string() };

                        let mem_usage = stats.memory_stats.usage.map_or("0 B".to_string(), format_size);

                        if let Some(networks) = stats.networks {
                            for (_, net_data) in networks {
                                current_stats.net_rx += net_data.rx_bytes;
                                current_stats.net_tx += net_data.tx_bytes;
                            }
                        }
                        let net_down_rate = format_rate((current_stats.net_rx.saturating_sub(prev_stats.net_rx) as f64 / elapsed_secs) as u64);
                        let net_up_rate = format_rate((current_stats.net_tx.saturating_sub(prev_stats.net_tx) as f64 / elapsed_secs) as u64);

                        if let Some(io_stats) = stats.blkio_stats.io_service_bytes_recursive {
                            for entry in io_stats {
                                if entry.op.as_str() == "Read" { current_stats.disk_r += entry.value; }
                                if entry.op.as_str() == "Write" { current_stats.disk_w += entry.value; }
                            }
                        }
                        let disk_r_rate = format_rate((current_stats.disk_r.saturating_sub(prev_stats.disk_r) as f64 / elapsed_secs) as u64);
                        let disk_w_rate = format_rate((current_stats.disk_w.saturating_sub(prev_stats.disk_w) as f64 / elapsed_secs) as u64);

                        current_container_stats_map.insert(id_full.clone(), current_stats);
                        (cpu_percent, mem_usage, net_down_rate, net_up_rate, disk_r_rate, disk_w_rate)
                    } else {
                        ("0.00%".to_string(), "0 B".to_string(), "0 B/s".to_string(), "0 B/s".to_string(), "0 B/s".to_string(), "0 B/s".to_string())
                    };

                    container_info_list.push(ContainerInfo { id, name, status, cpu, mem, net_down, net_up, disk_r, disk_w });
                }
                (container_info_list, current_container_stats_map)
            } else { (Vec::new(), HashMap::new()) }
        } else { (Vec::new(), HashMap::new()) }
    } else { (Vec::new(), HashMap::new()) };
    *prev_container_stats = current_container_stats;

    let gpus = match nvml {
        Ok(nvml_instance) => (|| {
            let mut gpu_infos = Vec::new();
            let device_count = nvml_instance.device_count().map_err(|e| e.to_string())?;
            for i in 0..device_count {
                let device = nvml_instance.device_by_index(i).map_err(|e| e.to_string())?;
                let mem = device.memory_info().map_err(|e| e.to_string())?;
                gpu_infos.push(GpuInfo {
                    name: device.name().map_err(|e| e.to_string())?,
                    brand: "NVIDIA".to_string(),
                    utilization: device.utilization_rates().map_err(|e| e.to_string())?.gpu,
                    memory_used: mem.used, memory_total: mem.total,
                    temperature: device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).map_err(|e| e.to_string())?,
                    power_usage: device.power_usage().map_err(|e| e.to_string())?,
                    graphics_clock: device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics).map_err(|e| e.to_string())?,
                    memory_clock: device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory).map_err(|e| e.to_string())?,
                });
            }
            Ok(gpu_infos)
        })(),
        Err(e) => Err(format!("NVML Error: {}", e)),
    };

    let gpu_util = if let Ok(gpu_data) = &gpus {
        gpu_data.first().map_or(0, |gpu| gpu.utilization)
    } else { 0 };

    let global_usage = GlobalUsage {
        cpu: sys.global_cpu_info().cpu_usage(),
        mem_used: sys.used_memory(),
        mem_total: sys.total_memory(),
        gpu_util,
        net_down: total_net_down,
        net_up: total_net_up,
        disk_read: total_disk_read,
        disk_write: total_disk_write,
    };

    DynamicData { processes, detailed_process, cores, disks, networks, containers, gpus, global_usage }
}


fn ui(f: &mut Frame, state: &mut AppState) {
    let tab_titles = ["Dashboard", "Process", "CPU Cores", "Disks", "Network", "GPU Details", "System"];
    let main_layout = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), // Title
        Constraint::Length(3), // Summary Bar
        Constraint::Min(0),    // Main content
        Constraint::Length(1)  // Footer
    ]).split(f.size());

    let tabs = Tabs::new(tab_titles.iter().cloned().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title("PULS").borders(Borders::ALL))
        .select(state.active_tab)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, main_layout[0]);

    draw_summary_bar(f, state, main_layout[1]);

    let inner_area = main_layout[2];
    match state.active_tab {
        0 => draw_dashboard_tab(f, state, inner_area),
        1 => draw_detailed_process_tab(f, state, inner_area),
        2 => draw_cpu_cores_tab(f, state, inner_area),
        3 => draw_disks_tab(f, state, inner_area),
        4 => draw_network_tab(f, state, inner_area),
        5 => draw_gpu_tab(f, state, inner_area),
        6 => draw_system_info_tab(f, state, inner_area),
        _ => {}
    }
    let footer_text = Paragraph::new("Quit: q | Tabs: Tab/Backtab | Select: ↓↑ | Details: Enter").style(Style::default().fg(Color::DarkGray)).alignment(Alignment::Center);
    f.render_widget(footer_text, main_layout[3]);
}

fn draw_summary_bar(f: &mut Frame, state: &mut AppState, area: Rect) {
    let usage = &state.dynamic_data.global_usage;
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(30), Constraint::Percentage(15), Constraint::Percentage(17), Constraint::Percentage(18)])
        .split(area);
    
    // CPU Usage
    let cpu_percent = usage.cpu;
    let cpu_gauge = Gauge::default()
        .block(Block::default().title("CPU Usage").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(cpu_percent as u16)
        .label(format!("{:.1}%", cpu_percent));
    f.render_widget(cpu_gauge, layout[0]);

    // Memory Usage
    let mem_percent = if usage.mem_total > 0 { (usage.mem_used as f64 / usage.mem_total as f64) * 100.0 } else { 0.0 };
    let mem_label = format!("{} / {}", format_size(usage.mem_used), format_size(usage.mem_total));
    let mem_gauge = Gauge::default()
        .block(Block::default().title("Memory Usage").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent(mem_percent as u16)
        .label(mem_label);
    f.render_widget(mem_gauge, layout[1]);

    // GPU Usage
    let gpu_percent = usage.gpu_util;
    let gpu_gauge = Gauge::default()
        .block(Block::default().title("GPU Usage").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Magenta).bg(Color::Black))
        .percent(gpu_percent as u16)
        .label(format!("{}%", gpu_percent));
    f.render_widget(gpu_gauge, layout[2]);

    // Network Usage
    let net_text = format!("▼ {}\n▲ {}", format_rate(usage.net_down), format_rate(usage.net_up));
    let net_paragraph = Paragraph::new(net_text)
        .block(Block::default().title("Network").borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(net_paragraph, layout[3]);

    // Disk I/O
    let disk_text = format!("▼ {}\n▲ {}", format_rate(usage.disk_read), format_rate(usage.disk_write));
    let disk_paragraph = Paragraph::new(disk_text)
        .block(Block::default().title("Disk I/O").borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(disk_paragraph, layout[4]);
}

fn draw_dashboard_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let main_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);

    let processes = &state.dynamic_data.processes;
    let process_headers = ["PID", "Name", "CPU %", "Memory", "Read/s", "Write/s"];
    let process_rows = processes.iter().map(|p| Row::new(vec![p.pid.clone(), p.name.clone(), p.cpu.clone(), p.mem.clone(), p.disk_read.clone(), p.disk_write.clone()]));
    let process_table = Table::new(process_rows, [Constraint::Length(8), Constraint::Min(15), Constraint::Length(8), Constraint::Length(10), Constraint::Length(12), Constraint::Length(12)])
        .header(Row::new(process_headers).style(Style::new().fg(Color::Cyan)).bottom_margin(1))
        .block(Block::default().title("Processes").borders(Borders::ALL))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");
    f.render_stateful_widget(process_table, main_layout[0], &mut state.process_table_state);

    let containers = &state.dynamic_data.containers;
    let container_headers = ["ID", "Name", "Status", "CPU %", "Memory", "Down/s", "Up/s", "Disk R/s", "Disk W/s"];
    let container_rows = containers.iter().map(|c| Row::new(vec![c.id.clone(), c.name.clone(), c.status.clone(), c.cpu.clone(), c.mem.clone(), c.net_down.clone(), c.net_up.clone(), c.disk_r.clone(), c.disk_w.clone()]));
    let container_table = Table::new(
        container_rows, 
        [Constraint::Length(14), Constraint::Min(15), Constraint::Min(10), Constraint::Length(8), Constraint::Length(10), Constraint::Length(12), Constraint::Length(12), Constraint::Length(12), Constraint::Length(12)]
    )
        .header(Row::new(container_headers).style(Style::new().fg(Color::Cyan)))
        .block(Block::default().title("Containers (Docker/Podman)").borders(Borders::ALL));
    f.render_widget(container_table, main_layout[1]);
}

fn draw_detailed_process_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let block = Block::default().title("Detailed Process Information").borders(Borders::ALL);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if let Some(p) = &state.dynamic_data.detailed_process {
        let layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner_area);
        let info_text = vec![
            Line::from(vec![Span::styled("PID: ", Style::default().fg(Color::Yellow)), Span::raw(p.pid.clone())]),
            Line::from(vec![Span::styled("Name: ", Style::default().fg(Color::Yellow)), Span::raw(p.name.clone())]),
            Line::from(vec![Span::styled("User: ", Style::default().fg(Color::Yellow)), Span::raw(p.user.clone())]),
            Line::from(vec![Span::styled("Parent: ", Style::default().fg(Color::Yellow)), Span::raw(p.parent.clone().unwrap_or_else(|| "N/A".into()))]),
            Line::from(vec![Span::styled("Status: ", Style::default().fg(Color::Yellow)), Span::raw(p.status.clone())]),
            Line::from(vec![Span::styled("Started: ", Style::default().fg(Color::Yellow)), Span::raw(p.start_time.clone())]),
            Line::from(vec![Span::styled("Memory (RSS): ", Style::default().fg(Color::Yellow)), Span::raw(format_size(p.memory_rss))]),
            Line::from(vec![Span::styled("Memory (VMS): ", Style::default().fg(Color::Yellow)), Span::raw(format_size(p.memory_vms))]),
            Line::from(vec![Span::styled("CPU Usage: ", Style::default().fg(Color::Yellow)), Span::raw(format!("{:.2}%", p.cpu_usage))]),
        ];
        f.render_widget(Paragraph::new(info_text).block(Block::default().title("Details").borders(Borders::ALL)), layout[0]);

        let cmd_and_env = vec![
            Line::from(Span::styled("Full Command:", Style::default().fg(Color::Yellow))),
            Line::from(Span::raw(p.command.clone())),
            Line::from(""),
            Line::from(Span::styled("Environment Variables:", Style::default().fg(Color::Yellow))),
        ].into_iter().chain(p.environ.iter().map(|e| Line::from(Span::raw(e.clone())))).collect::<Vec<_>>();
        f.render_widget(Paragraph::new(cmd_and_env).wrap(ratatui::widgets::Wrap{trim: false}).block(Block::default().title("Command & Environment").borders(Borders::ALL)), layout[1]);
    } else {
        f.render_widget(Paragraph::new("Select a process from the Dashboard with ↓↑ and press Enter to see details.").alignment(Alignment::Center).wrap(ratatui::widgets::Wrap{trim:true}), inner_area);
    }
}

fn draw_cpu_cores_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let block = Block::default().title("CPU Core Usage").borders(Borders::ALL);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let cores = &state.dynamic_data.cores;
    let num_cores = cores.len();
    if num_cores == 0 { return; }

    let constraints: Vec<Constraint> = cores.iter().map(|_| Constraint::Length(1)).collect();
    let layout = Layout::default().direction(Direction::Vertical).margin(1).constraints(constraints).split(inner_area);

    for (i, core) in cores.iter().enumerate() {
        if i >= layout.len() { break; }
        let label = format!("Core {} ({} MHz): {:.2}%", i, core.freq, core.usage);
        let gauge = Gauge::default()
            .label(label)
            .gauge_style(Style::default().fg(Color::Green))
            .ratio((core.usage / 100.0).into());
        f.render_widget(gauge, layout[i]);
    }
}

fn draw_disks_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let disks = &state.dynamic_data.disks;
    let headers = ["Mount Point", "FS", "Total", "Free"];
    let rows = disks.iter().map(|d| Row::new(vec![
        d.name.clone(), d.fs.clone(), format_size(d.total), format_size(d.free)
    ]));
    let table = Table::new(rows, [Constraint::Min(15), Constraint::Length(8), Constraint::Length(10), Constraint::Length(10)])
        .header(Row::new(headers).style(Style::new().fg(Color::Cyan)))
        .block(Block::default().title("Disk Usage").borders(Borders::ALL));
    f.render_widget(table, area);
}

fn draw_network_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let networks = &state.dynamic_data.networks;
    let headers = ["Interface", "Download/s", "Upload/s", "Total Down", "Total Up"];
    let rows = networks.iter().map(|n| Row::new(vec![
        n.name.clone(), format_rate(n.down_rate), format_rate(n.up_rate),
        format_size(n.total_down), format_size(n.total_up)
    ]));
    let table = Table::new(rows, [Constraint::Min(15), Constraint::Length(12), Constraint::Length(12), Constraint::Length(12), Constraint::Length(12)])
        .header(Row::new(headers).style(Style::new().fg(Color::Cyan)))
        .block(Block::default().title("Network Interfaces").borders(Borders::ALL));
    f.render_widget(table, area);
}

fn draw_system_info_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let rows = state.system_info.iter().map(|(key, val)| Row::new(vec![key.clone(), val.clone()]));
    let table = Table::new(rows, [Constraint::Length(15), Constraint::Min(30)])
        .header(Row::new(vec!["Component", "Information"]).style(Style::new().fg(Color::Cyan)))
        .block(Block::default().title("System Information").borders(Borders::ALL));
    f.render_widget(table, area);
}

fn draw_gpu_tab(f: &mut Frame, state: &mut AppState, area: Rect) {
    let gpu_block = Block::default().title("GPU Details").borders(Borders::ALL);
    let inner_area = gpu_block.inner(area);
    f.render_widget(gpu_block, area);

    match &state.dynamic_data.gpus {
        Ok(gpus) if gpus.is_empty() => { f.render_widget(Paragraph::new("No NVIDIA GPU found or NVML failed to initialize.").alignment(Alignment::Center), inner_area); }
        Ok(gpus) => {
            let num_gpus = gpus.len();
            if num_gpus == 0 { return; }
            let gpu_layout = Layout::default().direction(Direction::Vertical).margin(1).constraints(vec![Constraint::Ratio(1, num_gpus as u32); num_gpus]).split(inner_area);

            for (i, gpu) in gpus.iter().enumerate() {
                if i >= gpu_layout.len() { continue; }
                let gpu_area = gpu_layout[i];
                let title = format!("{} [{}] ({}°C)", gpu.name, gpu.brand, gpu.temperature);
                let details_block = Block::default().title(title).borders(Borders::ALL);
                let details_area = details_block.inner(gpu_area);
                f.render_widget(details_block, gpu_area);

                let block_layout = Layout::default().direction(Direction::Vertical).margin(1).constraints([Constraint::Length(1), Constraint::Min(0)]).split(details_area);
                let util_gauge = Gauge::default()
                    .label(format!("Core Usage: {}%", gpu.utilization))
                    .gauge_style(Style::default().fg(Color::Red))
                    .ratio((gpu.utilization as f64) / 100.0);
                f.render_widget(util_gauge, block_layout[0]);

                let mem_line = format!("{:.2} GB / {:.2} GB", gpu.memory_used as f64 / 1_073_741_824.0, gpu.memory_total as f64 / 1_073_741_824.0);
                let details_text: Vec<Line> = vec![
                    Line::from(vec![Span::styled("Memory:         ", Style::default().fg(Color::Yellow)), Span::raw(mem_line)]),
                    Line::from(vec![Span::styled("Power Usage:    ", Style::default().fg(Color::Yellow)), Span::raw(format!("{} W", gpu.power_usage / 1000))]),
                    Line::from(vec![Span::styled("Graphics Clock: ", Style::default().fg(Color::Yellow)), Span::raw(format!("{} MHz", gpu.graphics_clock))]),
                    Line::from(vec![Span::styled("Memory Clock:   ", Style::default().fg(Color::Yellow)), Span::raw(format!("{} MHz", gpu.memory_clock))]),
                ];
                f.render_widget(Paragraph::new(details_text), block_layout[1]);
            }
        }
        Err(e) => { f.render_widget(Paragraph::new(e.as_str()).alignment(Alignment::Center), inner_area); }
    }
}

fn format_rate(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{} B/s", bytes)
    } else if bytes < MB {
        format!("{:.1} KB/s", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB/s", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.1} GB/s", bytes as f64 / TB as f64)
    } else {
        format!("{:.1} TB/s", bytes as f64 / TB as f64)
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} TiB", bytes as f64 / TB as f64)
    }
}