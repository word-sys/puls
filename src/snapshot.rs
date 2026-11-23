use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::sys::unix_http::JsonValue;
use crate::types::{
    AppState, DynamicData, GlobalUsage, PressureStallInfo, ProcessInfo, PsiMetric, SystemTemperatures,
};
use crate::utils::{current_timestamp, format_size, format_timestamp};

pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn render_snapshot_json(system_info: &[(String, String)], data: &DynamicData) -> String {
    let ts = current_timestamp();
    let ts_str = format_timestamp(ts as i64, "%Y-%m-%d %H:%M:%S");
    let u = &data.global_usage;

    let mut out = String::with_capacity(4096);
    out.push_str("{\n");
    out.push_str(&format!("  \"timestamp\": {},\n", ts));
    out.push_str(&format!("  \"time_str\": {},\n", json_escape(&ts_str)));

    out.push_str("  \"system_info\": {\n");
    for (i, (k, v)) in system_info.iter().enumerate() {
        let comma = if i + 1 < system_info.len() { "," } else { "" };
        out.push_str(&format!("    {}: {}{}\n", json_escape(k), json_escape(v), comma));
    }
    out.push_str("  },\n");

    let cpu_temp_json = data
        .temperatures
        .cpu_temp
        .map(|t| format!("{:.1}", t))
        .unwrap_or_else(|| "null".to_string());
    let gpu_util_json = u
        .gpu_util
        .map(|g| g.to_string())
        .unwrap_or_else(|| "null".to_string());

    out.push_str(&format!(
        "  \"global\": {{\"cpu_pct\": {:.2}, \"mem_used\": {}, \"mem_total\": {}, \"mem_cached\": {}, \"swap_used\": {}, \"swap_total\": {}, \"load_avg\": [{:.2}, {:.2}, {:.2}], \"uptime\": {}, \"net_down\": {}, \"net_up\": {}, \"disk_read\": {}, \"disk_write\": {}, \"cpu_temp\": {}, \"gpu_util\": {}, \"psi_cpu_10s\": {:.2}, \"psi_mem_10s\": {:.2}, \"psi_io_10s\": {:.2}}},\n",
        u.cpu,
        u.mem_used,
        u.mem_total,
        u.mem_cached,
        u.swap_used,
        u.swap_total,
        u.load_average.0,
        u.load_average.1,
        u.load_average.2,
        u.uptime,
        u.net_down,
        u.net_up,
        u.disk_read,
        u.disk_write,
        cpu_temp_json,
        gpu_util_json,
        data.pressure.cpu.some_avg10,
        data.pressure.memory.some_avg10,
        data.pressure.io.some_avg10,
    ));

    out.push_str("  \"top_processes\": [");
    let top_procs: Vec<&ProcessInfo> = data.processes.iter().take(15).collect();
    for (i, p) in top_procs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n    {{\"pid\": {}, \"name\": {}, \"user\": {}, \"cpu\": {:.2}, \"mem\": {}, \"cgroup\": {}}}",
            json_escape(&p.pid),
            json_escape(&p.name),
            json_escape(&p.user),
            p.cpu,
            p.mem,
            p.cgroup_unit
                .as_deref()
                .map(json_escape)
                .unwrap_or_else(|| "null".to_string())
        ));
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"disks\": [");
    for (i, d) in data.disks.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n    {{\"mount\": {}, \"device\": {}, \"fs\": {}, \"used\": {}, \"total\": {}, \"await_ms\": {:.2}, \"busy_pct\": {:.1}}}",
            json_escape(&d.name),
            json_escape(&d.device),
            json_escape(&d.fs),
            d.used,
            d.total,
            d.await_ms,
            d.busy_pct
        ));
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"gpus\": [");
    if let Ok(ref gpus) = data.gpus {
        for (i, g) in gpus.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "\n    {{\"name\": {}, \"brand\": {}, \"util\": {}, \"mem_used\": {}, \"mem_total\": {}, \"temp\": {}, \"power_mw\": {}}}",
                json_escape(&g.name),
                json_escape(&g.brand),
                g.utilization,
                g.memory_used,
                g.memory_total,
                g.temperature,
                g.power_usage
            ));
        }
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"zram\": [");
    for (i, z) in data.zram_devices.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n    {{\"device\": {}, \"algo\": {}, \"orig_bytes\": {}, \"compr_bytes\": {}, \"ratio\": {:.2}}}",
            json_escape(&z.device),
            json_escape(&z.algorithm),
            z.orig_bytes,
            z.compr_bytes,
            z.ratio()
        ));
    }
    out.push_str("\n  ]\n}\n");

    out
}

pub fn render_snapshot_markdown(system_info: &[(String, String)], data: &DynamicData) -> String {
    let ts_str = format_timestamp(current_timestamp() as i64, "%Y-%m-%d %H:%M:%S");
    let u = &data.global_usage;
    let mut md = format!("# PULS System Snapshot ({})\n\n## System\n", ts_str);
    for (k, v) in system_info {
        md.push_str(&format!("- **{}**: {}\n", k, v));
    }
    md.push_str(&format!(
        "\n## Resources\n- **CPU**: {:.1}%\n- **Memory**: {} / {} (Cached: {})\n- **Swap**: {} / {}\n- **Load Avg**: {:.2}, {:.2}, {:.2}\n",
        u.cpu,
        format_size(u.mem_used),
        format_size(u.mem_total),
        format_size(u.mem_cached),
        format_size(u.swap_used),
        format_size(u.swap_total),
        u.load_average.0,
        u.load_average.1,
        u.load_average.2
    ));
    if data.pressure.available {
        md.push_str(&format!(
            "- **PSI (10s)**: CPU {:.1}% | Memory {:.1}% | I/O {:.1}%\n",
            data.pressure.cpu.some_avg10,
            data.pressure.memory.some_avg10,
            data.pressure.io.some_avg10
        ));
    }
    md.push_str("\n## Top Processes\n| PID | Name | User | CPU% | Memory |\n| :--- | :--- | :--- | ---: | ---: |\n");
    for p in data.processes.iter().take(10) {
        md.push_str(&format!(
            "| {} | {} | {} | {:.1}% | {} |\n",
            p.pid,
            p.name,
            p.user,
            p.cpu,
            format_size(p.mem)
        ));
    }
    md
}

#[derive(Clone, Debug)]
pub struct FlightFrame {
    pub ts: u64,
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub mem_cached: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub net_down: u64,
    pub net_up: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub load_avg: (f64, f64, f64),
    pub uptime: u64,
    pub cpu_temp: Option<f32>,
    pub gpu_util: Option<u32>,
    pub psi_cpu: f32,
    pub psi_mem: f32,
    pub psi_io: f32,
    pub processes: Vec<ProcessInfo>,
}

impl FlightFrame {
    pub fn from_dynamic_data(data: &DynamicData) -> Self {
        let u = &data.global_usage;
        Self {
            ts: current_timestamp(),
            cpu: u.cpu,
            mem_used: u.mem_used,
            mem_total: u.mem_total,
            mem_cached: u.mem_cached,
            swap_used: u.swap_used,
            swap_total: u.swap_total,
            net_down: u.net_down,
            net_up: u.net_up,
            disk_read: u.disk_read,
            disk_write: u.disk_write,
            load_avg: u.load_average,
            uptime: u.uptime,
            cpu_temp: data.temperatures.cpu_temp,
            gpu_util: u.gpu_util,
            psi_cpu: data.pressure.cpu.some_avg10,
            psi_mem: data.pressure.memory.some_avg10,
            psi_io: data.pressure.io.some_avg10,
            processes: data.processes.iter().take(15).cloned().collect(),
        }
    }

    pub fn to_jsonl_line(&self) -> String {
        let cpu_temp_json = self
            .cpu_temp
            .map(|t| format!("{:.1}", t))
            .unwrap_or_else(|| "null".to_string());
        let gpu_util_json = self
            .gpu_util
            .map(|g| g.to_string())
            .unwrap_or_else(|| "null".to_string());

        let mut procs_json = String::from("[");
        for (i, p) in self.processes.iter().enumerate() {
            if i > 0 {
                procs_json.push(',');
            }
            procs_json.push_str(&format!(
                "{{\"pid\":{},\"name\":{},\"user\":{},\"cpu\":{:.2},\"mem\":{},\"cg\":{}}}",
                json_escape(&p.pid),
                json_escape(&p.name),
                json_escape(&p.user),
                p.cpu,
                p.mem,
                p.cgroup_unit
                    .as_deref()
                    .map(json_escape)
                    .unwrap_or_else(|| "null".to_string())
            ));
        }
        procs_json.push(']');

        format!(
            "{{\"ts\":{},\"cpu\":{:.2},\"mem_used\":{},\"mem_total\":{},\"mem_cached\":{},\"swap_used\":{},\"swap_total\":{},\"net_down\":{},\"net_up\":{},\"disk_read\":{},\"disk_write\":{},\"l1\":{:.2},\"l5\":{:.2},\"l15\":{:.2},\"uptime\":{},\"cpu_temp\":{},\"gpu_util\":{},\"psi_cpu\":{:.2},\"psi_mem\":{:.2},\"psi_io\":{:.2},\"procs\":{}}}",
            self.ts,
            self.cpu,
            self.mem_used,
            self.mem_total,
            self.mem_cached,
            self.swap_used,
            self.swap_total,
            self.net_down,
            self.net_up,
            self.disk_read,
            self.disk_write,
            self.load_avg.0,
            self.load_avg.1,
            self.load_avg.2,
            self.uptime,
            cpu_temp_json,
            gpu_util_json,
            self.psi_cpu,
            self.psi_mem,
            self.psi_io,
            procs_json
        )
    }

    pub fn from_jsonl_line(line: &str) -> Result<Self, String> {
        let val = JsonValue::parse(line)?;
        let get_u64 = |k: &str| val.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
        let get_f64 = |k: &str| val.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);

        let mut processes = Vec::new();
        if let Some(arr) = val.get("procs").and_then(|v| v.as_array()) {
            for item in arr {
                processes.push(ProcessInfo {
                    pid: item.get("pid").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                    name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    user: item.get("user").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    cpu: item.get("cpu").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    mem: item.get("mem").and_then(|v| v.as_u64()).unwrap_or(0),
                    disk_read: 0,
                    disk_write: 0,
                    status: "Recorded".to_string(),
                    parent_pid: None,
                    tree_prefix: String::new(),
                    nice: 0,
                    cgroup_unit: item.get("cg").and_then(|v| v.as_str()).map(|s| s.to_string()),
                });
            }
        }

        Ok(Self {
            ts: get_u64("ts"),
            cpu: get_f64("cpu") as f32,
            mem_used: get_u64("mem_used"),
            mem_total: get_u64("mem_total"),
            mem_cached: get_u64("mem_cached"),
            swap_used: get_u64("swap_used"),
            swap_total: get_u64("swap_total"),
            net_down: get_u64("net_down"),
            net_up: get_u64("net_up"),
            disk_read: get_u64("disk_read"),
            disk_write: get_u64("disk_write"),
            load_avg: (get_f64("l1"), get_f64("l5"), get_f64("l15")),
            uptime: get_u64("uptime"),
            cpu_temp: val.get("cpu_temp").and_then(|v| v.as_f64()).map(|n| n as f32),
            gpu_util: val.get("gpu_util").and_then(|v| v.as_u64()).map(|n| n as u32),
            psi_cpu: get_f64("psi_cpu") as f32,
            psi_mem: get_f64("psi_mem") as f32,
            psi_io: get_f64("psi_io") as f32,
            processes,
        })
    }
}

pub fn append_flight_frame(path: &Path, data: &DynamicData) -> Result<(), String> {
    let frame = FlightFrame::from_dynamic_data(data);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed opening {}: {}", path.display(), e))?;
    writeln!(file, "{}", frame.to_jsonl_line())
        .map_err(|e| format!("Failed writing flight frame: {}", e))
}

pub fn load_flight_recording(path: &Path) -> Result<Vec<FlightFrame>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading {}: {}", path.display(), e))?;
    let mut frames = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(frame) = FlightFrame::from_jsonl_line(trimmed) {
            frames.push(frame);
        }
    }
    if frames.is_empty() {
        return Err(format!("No valid flight frames found in {}", path.display()));
    }
    Ok(frames)
}

pub fn apply_replay_frame(state: &mut AppState, frames: &[FlightFrame], idx: usize) {
    if frames.is_empty() {
        return;
    }
    let idx = idx.min(frames.len() - 1);
    let window_start = idx.saturating_sub(59);
    let slice = &frames[window_start..=idx];
    let current = &frames[idx];

    let mut cpu_history = VecDeque::with_capacity(60);
    let mut mem_history = VecDeque::with_capacity(60);
    let mut net_down_history = VecDeque::with_capacity(60);
    let mut net_up_history = VecDeque::with_capacity(60);
    let mut disk_read_history = VecDeque::with_capacity(60);
    let mut disk_write_history = VecDeque::with_capacity(60);
    let mut gpu_history = VecDeque::with_capacity(60);

    for f in slice {
        cpu_history.push_back(f.cpu);
        let mem_pct = if f.mem_total > 0 {
            (f.mem_used as f64 / f.mem_total as f64 * 100.0) as f32
        } else {
            0.0
        };
        mem_history.push_back(mem_pct);
        net_down_history.push_back(f.net_down);
        net_up_history.push_back(f.net_up);
        disk_read_history.push_back(f.disk_read);
        disk_write_history.push_back(f.disk_write);
        if let Some(g) = f.gpu_util {
            gpu_history.push_back(g);
        }
    }

    state.dynamic_data = DynamicData {
        processes: current.processes.clone(),
        global_usage: GlobalUsage {
            cpu: current.cpu,
            mem_used: current.mem_used,
            mem_total: current.mem_total,
            mem_cached: current.mem_cached,
            swap_used: current.swap_used,
            swap_total: current.swap_total,
            gpu_util: current.gpu_util,
            net_down: current.net_down,
            net_up: current.net_up,
            disk_read: current.disk_read,
            disk_write: current.disk_write,
            cpu_history,
            mem_history,
            net_down_history,
            net_up_history,
            disk_read_history,
            disk_write_history,
            gpu_history,
            load_average: current.load_avg,
            uptime: current.uptime,
            mem_details: None,
        },
        temperatures: SystemTemperatures {
            cpu_temp: current.cpu_temp,
        },
        pressure: PressureStallInfo {
            available: true,
            cpu: PsiMetric { some_avg10: current.psi_cpu, ..Default::default() },
            memory: PsiMetric { some_avg10: current.psi_mem, ..Default::default() },
            io: PsiMetric { some_avg10: current.psi_io, ..Default::default() },
        },
        ..Default::default()
    };

    let ts_str = format_timestamp(current.ts as i64, "%Y-%m-%d %H:%M:%S");
    state.replay_status = Some(format!(
        "REPLAY {}/{} @ {}  [</>: Scrub]",
        idx + 1,
        frames.len(),
        ts_str
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_json_and_markdown_render() {
        let sys_info = vec![
            ("OS".to_string(), "Ubuntu 24.04".to_string()),
            ("Hostname".to_string(), "puls-box".to_string()),
        ];
        let mut data = DynamicData::default();
        data.global_usage.cpu = 42.5;
        data.global_usage.mem_total = 16 * 1024 * 1024 * 1024;
        data.global_usage.mem_used = 8 * 1024 * 1024 * 1024;
        data.processes.push(ProcessInfo {
            pid: "123".to_string(),
            name: "puls".to_string(),
            cpu: 1.5,
            mem: 10 * 1024 * 1024,
            disk_read: 0,
            disk_write: 0,
            user: "root".to_string(),
            status: "Running".to_string(),
            parent_pid: None,
            tree_prefix: String::new(),
            nice: 0,
            cgroup_unit: Some("puls.service".to_string()),
        });

        let json_str = render_snapshot_json(&sys_info, &data);
        let parsed = JsonValue::parse(&json_str).expect("snapshot json must be valid JSON");
        assert_eq!(
            parsed.get("system_info").and_then(|s| s.get("Hostname")).and_then(|v| v.as_str()),
            Some("puls-box")
        );
        let md = render_snapshot_markdown(&sys_info, &data);
        assert!(md.contains("# PULS System Snapshot"));
        assert!(md.contains("| 123 | puls |"));
    }

    #[test]
    fn test_flight_recorder_jsonl_roundtrip() {
        let dir = std::env::temp_dir().join(format!("puls_flight_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("flight.jsonl");

        let mut data = DynamicData::default();
        data.global_usage.cpu = 33.3;
        data.global_usage.mem_total = 8192 * 1024 * 1024;
        data.global_usage.mem_used = 4096 * 1024 * 1024;
        data.temperatures.cpu_temp = Some(58.5);
        data.pressure.cpu.some_avg10 = 1.25;
        data.processes.push(ProcessInfo {
            pid: "999".to_string(),
            name: "worker \"alpha\"".to_string(),
            cpu: 12.5,
            mem: 50 * 1024 * 1024,
            disk_read: 0,
            disk_write: 0,
            user: "alice".to_string(),
            status: "Running".to_string(),
            parent_pid: None,
            tree_prefix: String::new(),
            nice: 0,
            cgroup_unit: Some("worker.service".to_string()),
        });

        append_flight_frame(&file_path, &data).expect("append frame 1");
        data.global_usage.cpu = 66.6;
        append_flight_frame(&file_path, &data).expect("append frame 2");

        let loaded = load_flight_recording(&file_path).expect("load recording");
        assert_eq!(loaded.len(), 2);
        assert!((loaded[0].cpu - 33.3).abs() < 0.1);
        assert!((loaded[1].cpu - 66.6).abs() < 0.1);
        assert_eq!(loaded[0].cpu_temp, Some(58.5));
        assert_eq!(loaded[0].processes[0].name, "worker \"alpha\"");

        let mut state = AppState::default();
        apply_replay_frame(&mut state, &loaded, 1);
        assert!((state.dynamic_data.global_usage.cpu - 66.6).abs() < 0.1);
        assert!(state.replay_status.as_deref().unwrap().contains("REPLAY 2/2"));

        let _ = fs::remove_dir_all(&dir);
    }
}
