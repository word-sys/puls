use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::sys::unix_http::{decode_docker_log_payload, JsonValue, UnixHttpClient};
use crate::types::{ContainerInfo, ContainerIoStats};
use crate::utils::{calculate_rate, format_rate, format_size, get_current_uid};

#[derive(Clone, Default, Debug)]
struct CgroupCpuSample {
    usage_usec: u64,
    timestamp: Option<Instant>,
}

pub struct ContainerMonitor {
    client: Option<UnixHttpClient>,
    pub init_error: Option<String>,
    prev_container_stats: HashMap<String, ContainerIoStats>,
    prev_cgroup_cpu: HashMap<String, CgroupCpuSample>,
    last_update: Instant,
}

impl ContainerMonitor {
    pub fn new() -> Self {
        let (client, init_error) = match Self::discover_client() {
            Ok(c) => (Some(c), None),
            Err(e) => (None, Some(e)),
        };

        Self {
            client,
            init_error,
            prev_container_stats: HashMap::new(),
            prev_cgroup_cpu: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    pub fn with_socket<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            client: Some(UnixHttpClient::new(socket_path)),
            init_error: None,
            prev_container_stats: HashMap::new(),
            prev_cgroup_cpu: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    fn discover_client() -> Result<UnixHttpClient, String> {
        let uid = get_current_uid();
        let mut candidates = Vec::new();

        if let Ok(host) = std::env::var("DOCKER_HOST") {
            if let Some(stripped) = host.strip_prefix("unix://") {
                candidates.push(PathBuf::from(stripped));
            }
        }

        candidates.push(PathBuf::from(format!("/run/user/{}/podman/podman.sock", uid)));
        candidates.push(PathBuf::from(format!("/run/user/{}/docker.sock", uid)));
        candidates.push(PathBuf::from("/var/run/docker.sock"));
        candidates.push(PathBuf::from("/run/podman/podman.sock"));

        let mut permission_denied_path = None;

        for path in candidates {
            if path.exists() {
                match std::os::unix::net::UnixStream::connect(&path) {
                    Ok(_) => return Ok(UnixHttpClient::new(path)),
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        permission_denied_path = Some(path);
                    }
                    Err(_) => {}
                }
            }
        }

        if let Some(p) = permission_denied_path {
            return Err(format!(
                "Permission denied accessing {}. Add user to 'docker' group or use rootless Podman.",
                p.display()
            ));
        }

        Err("Docker or Podman daemon socket not found.".to_string())
    }

    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }

    pub fn get_containers(&mut self, timeout_ms: u64) -> Result<Vec<ContainerInfo>, String> {
        let Some(client) = self.client.clone() else {
            return Err(self
                .init_error
                .clone()
                .unwrap_or_else(|| "Container runtime not available".to_string()));
        };

        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_update).as_secs_f64().max(0.1);
        self.last_update = now;

        let ping_timeout = Duration::from_millis((timeout_ms / 4).max(100));
        match client.get("/_ping", ping_timeout) {
            Ok(resp) if resp.status_code == 200 => {}
            Ok(resp) => {
                return Err(format!("Container daemon returned HTTP {}", resp.status_code));
            }
            Err(e) => {
                if e.contains("Permission denied") {
                    return Err(e);
                }
                return Err("Docker/Podman daemon is not running or not responding".to_string());
            }
        }

        let list_timeout = Duration::from_millis((timeout_ms / 2).max(150));
        let list_resp = client.get("/containers/json", list_timeout)?;
        if list_resp.status_code != 200 {
            return Err(format!("Failed to list containers (HTTP {})", list_resp.status_code));
        }

        let root_json = list_resp.json()?;
        let Some(containers_arr) = root_json.as_array() else {
            return Ok(Vec::new());
        };

        if containers_arr.is_empty() {
            return Ok(Vec::new());
        }

        let stats_timeout = Duration::from_millis((timeout_ms / 3).max(100));
        let mut container_infos = Vec::new();
        let mut current_container_stats = HashMap::new();

        for item in containers_arr {
            let id_full = item
                .get("Id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let id_short = id_full.get(..12).unwrap_or("N/A").to_string();

            let name = item
                .get("Names")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(|n| n.strip_prefix('/').unwrap_or(n).to_string())
                .or_else(|| item.get("Name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| "unnamed".to_string());

            let status = item
                .get("Status")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("State").and_then(|v| v.as_str()))
                .unwrap_or("unknown")
                .to_string();

            let image = item
                .get("Image")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let ports = Self::format_ports(item.get("Ports"));

            let (cpu, mem, net_down, net_up, disk_r, disk_w) = if let Some(metrics) =
                self.try_cgroup_v2_metrics(&id_full, elapsed_secs, &mut current_container_stats)
            {
                metrics
            } else {
                let stats_path = format!("/containers/{}/stats?stream=false", id_full);
                match client.get(&stats_path, stats_timeout).and_then(|r| r.json()) {
                    Ok(stats_json) => self.calculate_container_metrics_json(
                        &id_full,
                        &stats_json,
                        elapsed_secs,
                        &mut current_container_stats,
                    ),
                    Err(e) => {
                        crate::puls_warn!("Failed to get stats for container {}: {}", id_short, e);
                        (
                            "0.00%".to_string(),
                            "0 B".to_string(),
                            "0 B/s".to_string(),
                            "0 B/s".to_string(),
                            "0 B/s".to_string(),
                            "0 B/s".to_string(),
                        )
                    }
                }
            };

            container_infos.push(ContainerInfo {
                id: id_short,
                name,
                status,
                cpu,
                mem,
                net_down,
                net_up,
                disk_r,
                disk_w,
                image,
                ports,
            });
        }

        self.prev_container_stats = current_container_stats;
        Ok(container_infos)
    }

    fn try_cgroup_v2_metrics(
        &mut self,
        id_full: &str,
        elapsed_secs: f64,
        current_stats: &mut HashMap<String, ContainerIoStats>,
    ) -> Option<(String, String, String, String, String, String)> {
        if id_full.len() < 12 {
            return None;
        }
        let uid = get_current_uid();
        let candidate_dirs = [
            format!("/sys/fs/cgroup/system.slice/docker-{}.scope", id_full),
            format!("/sys/fs/cgroup/system.slice/libpod-{}.scope", id_full),
            format!(
                "/sys/fs/cgroup/user.slice/user-{}.slice/user@{}.service/user.slice/libpod-{}.scope",
                uid, uid, id_full
            ),
            format!(
                "/sys/fs/cgroup/user.slice/user-{}.slice/user@{}.service/docker-{}.scope",
                uid, uid, id_full
            ),
            format!("/sys/fs/cgroup/docker/{}", id_full),
        ];

        let cgroup_dir = candidate_dirs
            .iter()
            .map(PathBuf::from)
            .find(|p| p.join("cpu.stat").exists() && p.join("memory.current").exists())?;

        let cpu_stat_str = fs::read_to_string(cgroup_dir.join("cpu.stat")).ok()?;
        let mem_current_str = fs::read_to_string(cgroup_dir.join("memory.current")).ok()?;

        let usage_usec = parse_cgroup_cpu_usage_usec(&cpu_stat_str)?;
        let raw_mem = mem_current_str.trim().parse::<u64>().ok()?;
        let inactive_file = fs::read_to_string(cgroup_dir.join("memory.stat"))
            .ok()
            .and_then(|s| parse_cgroup_key_val(&s, "inactive_file"))
            .unwrap_or(0);
        let working_mem = raw_mem.saturating_sub(inactive_file);

        let now = Instant::now();
        let cpu_pct = if let Some(prev) = self.prev_cgroup_cpu.get(id_full) {
            if let Some(prev_ts) = prev.timestamp {
                let dt_usec = now.duration_since(prev_ts).as_micros() as f64;
                if dt_usec > 0.0 && usage_usec >= prev.usage_usec {
                    ((usage_usec - prev.usage_usec) as f64 / dt_usec) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        self.prev_cgroup_cpu.insert(
            id_full.to_string(),
            CgroupCpuSample {
                usage_usec,
                timestamp: Some(now),
            },
        );

        let (disk_r, disk_w) = fs::read_to_string(cgroup_dir.join("io.stat"))
            .ok()
            .map(|s| parse_cgroup_io_stat(&s))
            .unwrap_or((0, 0));

        let prev_io = self
            .prev_container_stats
            .get(id_full)
            .cloned()
            .unwrap_or_default();

        let disk_read_rate = calculate_rate(disk_r, prev_io.disk_r, elapsed_secs);
        let disk_write_rate = calculate_rate(disk_w, prev_io.disk_w, elapsed_secs);

        current_stats.insert(
            id_full.to_string(),
            ContainerIoStats {
                net_rx: prev_io.net_rx,
                net_tx: prev_io.net_tx,
                disk_r,
                disk_w,
            },
        );

        Some((
            format!("{:.2}%", cpu_pct),
            format_size(working_mem),
            "0 B/s".to_string(),
            "0 B/s".to_string(),
            format_rate(disk_read_rate),
            format_rate(disk_write_rate),
        ))
    }

    pub fn calculate_container_metrics_json(
        &self,
        container_id: &str,
        stats: &JsonValue,
        elapsed_secs: f64,
        current_stats: &mut HashMap<String, ContainerIoStats>,
    ) -> (String, String, String, String, String, String) {
        let prev_stats = self
            .prev_container_stats
            .get(container_id)
            .cloned()
            .unwrap_or_default();

        let mut container_io_stats = ContainerIoStats::default();

        let cpu_usage = Self::calculate_cpu_usage_json(stats);
        let cpu_display = format!("{:.2}%", cpu_usage);

        let memory_usage = stats
            .get("memory_stats")
            .and_then(|m| {
                let usage = m.get("usage").and_then(|v| v.as_u64()).unwrap_or(0);
                let inactive_file = m
                    .get("stats")
                    .and_then(|s| s.get("inactive_file").or_else(|| s.get("total_inactive_file")))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                Some(usage.saturating_sub(inactive_file))
            })
            .unwrap_or(0);
        let memory_display = format_size(memory_usage);

        if let Some(networks) = stats.get("networks").and_then(|n| n.as_object()) {
            for net_data in networks.values() {
                container_io_stats.net_rx += net_data.get("rx_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                container_io_stats.net_tx += net_data.get("tx_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }

        let net_rx_rate = calculate_rate(container_io_stats.net_rx, prev_stats.net_rx, elapsed_secs);
        let net_tx_rate = calculate_rate(container_io_stats.net_tx, prev_stats.net_tx, elapsed_secs);

        if let Some(entries) = stats
            .get("blkio_stats")
            .and_then(|b| b.get("io_service_bytes_recursive"))
            .and_then(|e| e.as_array())
        {
            for entry in entries {
                let op = entry.get("op").and_then(|v| v.as_str()).unwrap_or("");
                let value = entry.get("value").and_then(|v| v.as_u64()).unwrap_or(0);
                if op.eq_ignore_ascii_case("read") {
                    container_io_stats.disk_r += value;
                } else if op.eq_ignore_ascii_case("write") {
                    container_io_stats.disk_w += value;
                }
            }
        }

        let disk_read_rate = calculate_rate(container_io_stats.disk_r, prev_stats.disk_r, elapsed_secs);
        let disk_write_rate = calculate_rate(container_io_stats.disk_w, prev_stats.disk_w, elapsed_secs);

        current_stats.insert(container_id.to_string(), container_io_stats);

        (
            cpu_display,
            memory_display,
            format_rate(net_rx_rate),
            format_rate(net_tx_rate),
            format_rate(disk_read_rate),
            format_rate(disk_write_rate),
        )
    }

    fn calculate_cpu_usage_json(stats: &JsonValue) -> f64 {
        let Some(cpu_stats) = stats.get("cpu_stats") else {
            return 0.0;
        };
        let Some(precpu_stats) = stats.get("precpu_stats") else {
            return 0.0;
        };

        let cpu_usage = cpu_stats
            .get("cpu_usage")
            .and_then(|u| u.get("total_usage"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let precpu_usage = precpu_stats
            .get("cpu_usage")
            .and_then(|u| u.get("total_usage"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let system_usage = cpu_stats
            .get("system_cpu_usage")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let presystem_usage = precpu_stats
            .get("system_cpu_usage")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cpu_delta = cpu_usage.saturating_sub(precpu_usage) as f64;
        let system_delta = system_usage.saturating_sub(presystem_usage) as f64;

        let num_cpus = cpu_stats
            .get("online_cpus")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                cpu_stats
                    .get("cpu_usage")
                    .and_then(|u| u.get("percpu_usage"))
                    .and_then(|a| a.as_array())
                    .map(|a| a.len() as u64)
            })
            .unwrap_or(1)
            .max(1) as f64;

        if system_delta > 0.0 && cpu_delta > 0.0 {
            (cpu_delta / system_delta) * num_cpus * 100.0
        } else {
            0.0
        }
    }

    fn format_ports(ports_val: Option<&JsonValue>) -> String {
        let Some(ports) = ports_val.and_then(|v| v.as_array()) else {
            return "none".to_string();
        };

        let mut port_strings = Vec::new();
        for port in ports {
            let private_port = port.get("PrivatePort").and_then(|v| v.as_u16());
            let public_port = port.get("PublicPort").and_then(|v| v.as_u16());
            match (public_port, private_port) {
                (Some(pub_p), Some(priv_p)) => port_strings.push(format!("{}:{}", pub_p, priv_p)),
                (None, Some(priv_p)) => port_strings.push(format!("{}", priv_p)),
                _ => {}
            }
        }

        if port_strings.is_empty() {
            "none".to_string()
        } else {
            port_strings.sort();
            port_strings.dedup();
            port_strings.join(", ")
        }
    }

    fn run_container_action(&self, container_id: &str, action: &str) -> Result<(), String> {
        if let Some(ref client) = self.client {
            let path = format!("/containers/{}/{}", container_id, action);
            match client.post(&path, &[], Duration::from_secs(10)) {
                Ok(resp) if (200..300).contains(&resp.status_code) || resp.status_code == 304 => {
                    return Ok(());
                }
                Ok(resp) => {
                    let msg = resp
                        .json()
                        .ok()
                        .and_then(|j| j.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| resp.body_string());
                    return Err(format!("Container {} failed (HTTP {}): {}", action, resp.status_code, msg.trim()));
                }
                Err(e) => {
                    crate::puls_warn!("Socket action {} failed, trying CLI fallback: {}", action, e);
                }
            }
        }

        for bin in ["docker", "podman"] {
            if let Ok(out) = std::process::Command::new(bin)
                .args([action, container_id])
                .output()
            {
                if out.status.success() {
                    return Ok(());
                } else {
                    return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
                }
            }
        }

        Err(format!("Failed to execute '{}' on container {}", action, container_id))
    }

    pub fn start_container(&self, container_id: &str) -> Result<(), String> {
        self.run_container_action(container_id, "start")
    }

    pub fn stop_container(&self, container_id: &str) -> Result<(), String> {
        self.run_container_action(container_id, "stop")
    }

    pub fn restart_container(&self, container_id: &str) -> Result<(), String> {
        self.run_container_action(container_id, "restart")
    }

    pub fn pause_container(&self, container_id: &str) -> Result<(), String> {
        self.run_container_action(container_id, "pause")
    }

    pub fn unpause_container(&self, container_id: &str) -> Result<(), String> {
        self.run_container_action(container_id, "unpause")
    }

    pub fn get_container_logs(&self, container_id: &str) -> Result<Vec<String>, String> {
        if let Some(ref client) = self.client {
            let path = format!(
                "/containers/{}/logs?stdout=true&stderr=true&tail=100",
                container_id
            );
            if let Ok(resp) = client.get(&path, Duration::from_secs(5)) {
                if resp.status_code == 200 {
                    let lines = decode_docker_log_payload(&resp.body);
                    return Ok(lines);
                }
            }
        }

        for bin in ["docker", "podman"] {
            if let Ok(out) = std::process::Command::new(bin)
                .args(["logs", "--tail", "100", container_id])
                .output()
            {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
                    if !stderr.is_empty() {
                        lines.extend(stderr.lines().map(|s| s.to_string()));
                    }
                    return Ok(lines);
                }
            }
        }

        Err(format!("Failed to retrieve logs for container {}", container_id))
    }
}

pub fn parse_cgroup_key_val(content: &str, key: &str) -> Option<u64> {
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some(key) {
            return parts.next().and_then(|v| v.parse::<u64>().ok());
        }
    }
    None
}

pub fn parse_cgroup_cpu_usage_usec(content: &str) -> Option<u64> {
    parse_cgroup_key_val(content, "usage_usec")
}

pub fn parse_cgroup_io_stat(content: &str) -> (u64, u64) {
    let mut rbytes = 0u64;
    let mut wbytes = 0u64;
    for line in content.lines() {
        for token in line.split_whitespace().skip(1) {
            if let Some((k, v)) = token.split_once('=') {
                if let Ok(val) = v.parse::<u64>() {
                    if k == "rbytes" {
                        rbytes += val;
                    } else if k == "wbytes" {
                        wbytes += val;
                    }
                }
            }
        }
    }
    (rbytes, wbytes)
}

impl Default for ContainerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;

    #[test]
    fn test_container_monitor_creation() {
        let _monitor = ContainerMonitor::new();
    }

    #[test]
    fn test_parse_cgroup_v2_helpers() {
        let cpu_stat = "usage_usec 2450000\nuser_usec 1400000\nsystem_usec 1050000\n";
        assert_eq!(parse_cgroup_cpu_usage_usec(cpu_stat), Some(2_450_000));

        let io_stat = "259:0 rbytes=1048576 wbytes=2097152 rios=100 wios=200\n259:1 rbytes=4096 wbytes=8192\n";
        assert_eq!(parse_cgroup_io_stat(io_stat), (1_052_672, 2_105_344));
    }

    #[test]
    fn test_mock_docker_socket_list_and_stats() {
        let sock_path = std::env::temp_dir().join(format!("puls_docker_mock_{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock_path);
        let listener = UnixListener::bind(&sock_path).expect("bind mock docker socket");

        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 1024];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let body = if req.starts_with("GET /_ping") {
                        "OK"
                    } else if req.starts_with("GET /containers/json") {
                        r#"[{"Id":"1234567890abcdef1234","Names":["/puls-web"],"Status":"Up 2 hours","Image":"nginx:alpine","Ports":[{"PrivatePort":80,"PublicPort":8080}]}]"#
                    } else {
                        r#"{"cpu_stats":{"cpu_usage":{"total_usage":200000000},"system_cpu_usage":1000000000,"online_cpus":4},"precpu_stats":{"cpu_usage":{"total_usage":100000000},"system_cpu_usage":500000000},"memory_stats":{"usage":52428800},"networks":{"eth0":{"rx_bytes":10000,"tx_bytes":5000}},"blkio_stats":{"io_service_bytes_recursive":[{"op":"Read","value":4096},{"op":"Write","value":8192}]}}"#
                    };
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes());
                }
            }
        });

        let mut monitor = ContainerMonitor::with_socket(&sock_path);
        let list = monitor.get_containers(1000).expect("get_containers failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "1234567890ab");
        assert_eq!(list[0].name, "puls-web");
        assert_eq!(list[0].image, "nginx:alpine");
        assert_eq!(list[0].ports, "8080:80");
        assert_eq!(list[0].mem, "50.0 MiB");
        assert_eq!(list[0].cpu, "80.00%");

        let _ = server.join();
        let _ = std::fs::remove_file(&sock_path);
    }

    #[test]
    fn test_container_actions_call() {
        let monitor = ContainerMonitor::with_socket("/tmp/puls_nonexistent_test.sock");
        let _ = monitor.start_container("puls_test_nonexistent");
        let _ = monitor.stop_container("puls_test_nonexistent");
        let _ = monitor.restart_container("puls_test_nonexistent");
        let _ = monitor.pause_container("puls_test_nonexistent");
        let _ = monitor.unpause_container("puls_test_nonexistent");
        let _ = monitor.get_container_logs("puls_test_nonexistent");
    }
}
