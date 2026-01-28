use std::collections::VecDeque;
use sysinfo::Pid;
use ratatui::widgets::TableState;

#[derive(Clone, Default, Debug)]
pub struct NetworkStats {
    pub rx: u64,
    pub tx: u64,
}

#[derive(Clone, Default, Debug)]
pub struct ContainerIoStats {
    pub net_rx: u64,
    pub net_tx: u64,
    pub disk_r: u64,
    pub disk_w: u64,
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: String,
    pub name: String,
    pub cpu: f32,           
    pub cpu_display: String, 
    pub mem: u64,           
    pub mem_display: String, 
    pub disk_read: String,
    pub disk_write: String,
    pub user: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub cpu: String,
    pub mem: String,
    pub net_down: String,
    pub net_up: String,
    pub disk_r: String,
    pub disk_w: String,
    pub image: String,
    pub ports: String,
}

#[derive(Clone, Debug, Default)]
pub struct GpuInfo {
    pub name: String,
    pub brand: String,
    pub utilization: u32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature: u32,
    pub memory_temperature: Option<u32>,
    pub power_usage: u32,
    pub graphics_clock: u32,
    pub memory_clock: u32,
    pub fan_speed: Option<u32>,
    pub pci_link_gen: Option<u32>,
    pub pci_link_width: Option<u32>,
    pub driver_version: String,
}

#[derive(Clone, Debug, Default)]
pub struct DetailedProcessInfo {
    pub pid: String,
    pub name: String,
    pub user: String,
    pub status: String,
    pub cpu_usage: f32,
    pub memory_rss: u64,
    pub memory_vms: u64,
    pub command: String,
    pub start_time: String,
    pub parent: Option<String>,
    pub environ: Vec<String>,
    pub threads: u32,
    pub file_descriptors: Option<u32>,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CoreInfo {
    pub usage: f32,
    pub freq: u64,
    pub temp: Option<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct DetailedDiskInfo {
    pub name: String,
    pub device: String,
    pub fs: String,
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub read_rate: u64,
    pub write_rate: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub is_ssd: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct DetailedNetInfo {
    pub name: String,
    pub down_rate: u64,
    pub up_rate: u64,
    pub total_down: u64,
    pub total_up: u64,
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub errors_rx: u64,
    pub errors_tx: u64,
    pub interface_type: String,
    pub is_up: bool,
}

#[derive(Clone, Debug)]
pub struct SystemTemperatures {
    pub cpu_temp: Option<f32>,
    pub gpu_temps: Vec<f32>,
    pub motherboard_temp: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct GlobalUsage {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub mem_cached: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub gpu_util: Option<u32>,
    pub net_down: u64,
    pub net_up: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub disk_read_ops: u64,
    pub disk_write_ops: u64,
    pub cpu_history: VecDeque<f32>,
    pub mem_history: VecDeque<f32>,
    pub net_down_history: VecDeque<u64>,
    pub net_up_history: VecDeque<u64>,
    pub disk_read_history: VecDeque<u64>,
    pub disk_write_history: VecDeque<u64>,
    pub gpu_history: VecDeque<u32>,
    pub load_average: (f64, f64, f64),
    pub uptime: u64,
    pub boot_time: u64,
}

impl Default for GlobalUsage {
    fn default() -> Self {
        Self {
            cpu: 0.0,
            mem_used: 0,
            mem_total: 0,
            mem_cached: 0,
            swap_used: 0,
            swap_total: 0,
            gpu_util: None,
            net_down: 0,
            net_up: 0,
            disk_read: 0,
            disk_write: 0,
            disk_read_ops: 0,
            disk_write_ops: 0,
            cpu_history: VecDeque::from(vec![0.0; 60]),
            mem_history: VecDeque::from(vec![0.0; 60]),
            net_down_history: VecDeque::from(vec![0; 60]),
            net_up_history: VecDeque::from(vec![0; 60]),
            disk_read_history: VecDeque::from(vec![0; 60]),
            disk_write_history: VecDeque::from(vec![0; 60]),
            gpu_history: VecDeque::from(vec![0; 60]),
            load_average: (0.0, 0.0, 0.0),
            uptime: 0,
            boot_time: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DynamicData {
    pub processes: Vec<ProcessInfo>,
    pub detailed_process: Option<DetailedProcessInfo>,
    pub cores: Vec<CoreInfo>,
    pub disks: Vec<DetailedDiskInfo>,
    pub networks: Vec<DetailedNetInfo>,
    pub containers: Vec<ContainerInfo>,
    pub gpus: Result<Vec<GpuInfo>, String>,
    pub global_usage: GlobalUsage,
    pub temperatures: SystemTemperatures,
    pub last_update: std::time::Instant,
    pub docker_error: Option<String>,
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
            temperatures: SystemTemperatures {
                cpu_temp: None,
                gpu_temps: Vec::new(),
                motherboard_temp: None,
            },
            last_update: std::time::Instant::now(),
            docker_error: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BootInfo {
    pub id: String,
    pub timestamp: String,
}

#[derive(Clone, Default)]
pub struct AppState {
    pub active_tab: usize,
    pub process_table_state: TableState,
    pub container_table_state: TableState,
    pub services_table_state: TableState,
    pub logs_table_state: TableState,
    pub config_table_state: TableState,
    pub selected_pid: Option<Pid>,
    pub system_info: Vec<(String, String)>,
    pub dynamic_data: DynamicData,
    pub sort_by: ProcessSortBy,
    pub sort_ascending: bool,
    pub filter_text: String,
    pub show_system_processes: bool,
    pub paused: bool,
    pub services: Vec<ServiceInfo>,
    pub logs: Vec<LogEntry>,
    pub boots: Vec<BootInfo>,
    pub current_boot_idx: usize,
    pub config_items: Vec<ConfigItem>,
    pub editing_service: Option<usize>,
    pub editing_config: Option<usize>,
    pub edit_buffer: String,
    pub has_sudo: bool,
    pub log_filter: String,
    pub service_status_modal: Option<(String, String)>,
    pub editing_filter: bool,
    pub docker_error: Option<String>,
    pub current_theme: usize,
    pub pending_kill_pid: Option<sysinfo::Pid>,
}

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub status: String,
    pub enabled: bool,
    pub can_start: bool,
    pub can_stop: bool,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            status: "unknown".to_string(),
            enabled: false,
            can_start: false,
            can_stop: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub service: String,
    pub message: String,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            timestamp: String::new(),
            level: String::new(),
            service: String::new(),
            message: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
    pub description: String,
    pub category: String,
}

impl Default for ConfigItem {
    fn default() -> Self {
        Self {
            key: String::new(),
            value: String::new(),
            description: String::new(),
            category: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessSortBy {
    Cpu,
    Memory,
    Name,
    Pid,
    DiskRead,
    DiskWrite,
}

impl Default for ProcessSortBy {
    fn default() -> Self {
        ProcessSortBy::Cpu
    }
}

#[derive(Clone, Debug)]
pub enum AppMessage {
    UpdateData(DynamicData),
    Error(String),
    TogglePause,
}

#[derive(Clone, Debug, Default)]
pub struct AppPerformance {
    pub update_duration_ms: u64,
    pub render_duration_ms: u64,
    pub memory_usage: u64,
    pub fps: f32,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub safe_mode: bool,
    pub refresh_rate_ms: u64,
    pub history_length: usize,
    pub enable_docker: bool,
    pub enable_gpu_monitoring: bool,
    pub enable_network_monitoring: bool,
    pub show_system_processes: bool,
    pub auto_scroll: bool,
    pub language: crate::language::Language,
}