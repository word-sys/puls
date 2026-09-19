use std::fs;
use std::path::Path;
use crate::types::NumaNodeInfo;

pub struct NumaMonitor;

impl NumaMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_numa_nodes(&self) -> Vec<NumaNodeInfo> {
        let node_root = Path::new("/sys/devices/system/node");
        if !node_root.exists() {
            return Vec::new();
        }

        let mut nodes = Vec::new();

        if let Ok(entries) = fs::read_dir(node_root) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();
                if name_str.starts_with("node") && name_str.len() > 4 && name_str[4..].chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(id) = name_str[4..].parse::<usize>() {
                        let node_dir = entry.path();
                        let cpu_list_str = fs::read_to_string(node_dir.join("cpulist"))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        let cpus = parse_cpulist(&cpu_list_str);

                        let (mem_total_bytes, mem_used_bytes, mem_free_bytes) = parse_meminfo(&node_dir.join("meminfo"), id);
                        let (numa_hit, numa_miss) = parse_numastat(&node_dir.join("numastat"));

                        nodes.push(NumaNodeInfo {
                            id,
                            name: format!("Node {}", id),
                            cpus,
                            cpu_list_str,
                            mem_total_bytes,
                            mem_used_bytes,
                            mem_free_bytes,
                            numa_hit,
                            numa_miss,
                        });
                    }
                }
            }
        }

        nodes.sort_by_key(|n| n.id);
        nodes
    }
}

impl Default for NumaMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_cpulist(s: &str) -> Vec<usize> {
    let mut cpus = Vec::new();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return cpus;
    }

    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_str, end_str)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start_str.trim().parse::<usize>(), end_str.trim().parse::<usize>()) {
                if start <= end {
                    cpus.extend(start..=end);
                }
            }
        } else if let Ok(cpu) = part.parse::<usize>() {
            cpus.push(cpu);
        }
    }
    cpus.sort_unstable();
    cpus.dedup();
    cpus
}

pub fn parse_meminfo_str(content: &str, node_id: usize) -> (u64, u64, u64) {
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    let mut used_kb: Option<u64> = None;

    let prefix_total = format!("Node {} MemTotal:", node_id);
    let prefix_free = format!("Node {} MemFree:", node_id);
    let prefix_used = format!("Node {} MemUsed:", node_id);

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&prefix_total) {
            if let Some(val) = extract_kb_val(trimmed) {
                total_kb = val;
            }
        } else if trimmed.starts_with(&prefix_free) {
            if let Some(val) = extract_kb_val(trimmed) {
                free_kb = val;
            }
        } else if trimmed.starts_with(&prefix_used) {
            if let Some(val) = extract_kb_val(trimmed) {
                used_kb = Some(val);
            }
        }
    }

    let used_final = used_kb.unwrap_or_else(|| total_kb.saturating_sub(free_kb));
    (total_kb * 1024, used_final * 1024, free_kb * 1024)
}

fn parse_meminfo(path: &Path, node_id: usize) -> (u64, u64, u64) {
    if let Ok(content) = fs::read_to_string(path) {
        parse_meminfo_str(&content, node_id)
    } else {
        (0, 0, 0)
    }
}

fn extract_kb_val(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        parts[3].parse::<u64>().ok()
    } else {
        None
    }
}

pub fn parse_numastat_str(content: &str) -> (Option<u64>, Option<u64>) {
    let mut hit = None;
    let mut miss = None;

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("numa_hit"), Some(v)) => hit = v.parse::<u64>().ok(),
            (Some("numa_miss"), Some(v)) => miss = v.parse::<u64>().ok(),
            _ => {}
        }
    }
    (hit, miss)
}

fn parse_numastat(path: &Path) -> (Option<u64>, Option<u64>) {
    if let Ok(content) = fs::read_to_string(path) {
        parse_numastat_str(&content)
    } else {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpulist_ranges() {
        assert_eq!(parse_cpulist("0-3,8-11"), vec![0, 1, 2, 3, 8, 9, 10, 11]);
        assert_eq!(parse_cpulist("0,2,4,6"), vec![0, 2, 4, 6]);
        assert_eq!(parse_cpulist("0-1,3,5-7"), vec![0, 1, 3, 5, 6, 7]);
        assert_eq!(parse_cpulist("4"), vec![4]);
        assert_eq!(parse_cpulist(""), Vec::<usize>::new());
        assert_eq!(parse_cpulist("  "), Vec::<usize>::new());
    }

    #[test]
    fn test_parse_meminfo_synthetic() {
        let content = "\
Node 0 MemTotal:       16384000 kB
Node 0 MemFree:         4096000 kB
Node 0 MemUsed:        12288000 kB
Node 0 Active:          8000000 kB
";
        let (total, used, free) = parse_meminfo_str(content, 0);
        assert_eq!(total, 16384000 * 1024);
        assert_eq!(used, 12288000 * 1024);
        assert_eq!(free, 4096000 * 1024);
    }

    #[test]
    fn test_parse_numastat_synthetic() {
        let content = "\
numa_hit 73757610
numa_miss 1234
numa_foreign 0
";
        let (hit, miss) = parse_numastat_str(content);
        assert_eq!(hit, Some(73757610));
        assert_eq!(miss, Some(1234));
    }

    #[test]
    fn test_numa_monitor_query() {
        let monitor = NumaMonitor::new();
        let nodes = monitor.get_numa_nodes();
        // On our Linux host with NUMA node0, this will return at least 1 node
        if !nodes.is_empty() {
            let n0 = &nodes[0];
            assert_eq!(n0.id, 0);
            assert!(!n0.cpus.is_empty());
            assert!(n0.mem_total_bytes > 0);
        }
    }
}
