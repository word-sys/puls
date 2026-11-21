use std::collections::HashSet;
use std::fs;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTicks {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuTicks {
    pub fn total(&self) -> u64 {
        self.user
            .saturating_add(self.nice)
            .saturating_add(self.system)
            .saturating_add(self.idle)
            .saturating_add(self.iowait)
            .saturating_add(self.irq)
            .saturating_add(self.softirq)
            .saturating_add(self.steal)
    }

    pub fn idle_all(&self) -> u64 {
        self.idle.saturating_add(self.iowait)
    }

    pub fn usage_percent(&self, prev: &CpuTicks) -> f32 {
        let total_delta = self.total().saturating_sub(prev.total());
        if total_delta == 0 {
            return 0.0;
        }
        let idle_delta = self.idle_all().saturating_sub(prev.idle_all());
        let active_delta = total_delta.saturating_sub(idle_delta);
        ((active_delta as f64 / total_delta as f64) * 100.0).clamp(0.0, 100.0) as f32
    }
}

fn parse_cpu_line(parts: &[&str]) -> Option<CpuTicks> {
    if parts.len() < 5 {
        return None;
    }
    Some(CpuTicks {
        user: parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
        nice: parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
        system: parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
        idle: parts.get(4).and_then(|v| v.parse().ok()).unwrap_or(0),
        iowait: parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0),
        irq: parts.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
        softirq: parts.get(7).and_then(|v| v.parse().ok()).unwrap_or(0),
        steal: parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0),
    })
}

pub fn parse_proc_stat(content: &str) -> (CpuTicks, Vec<CpuTicks>, u64) {
    let mut global = CpuTicks::default();
    let mut cores = Vec::new();
    let mut btime = 0u64;

    for line in content.lines() {
        if line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(t) = parse_cpu_line(&parts) {
                global = t;
            }
        } else if line.starts_with("cpu") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(label) = parts.first() {
                if let Ok(core_idx) = label[3..].parse::<usize>() {
                    if let Some(t) = parse_cpu_line(&parts) {
                        cores.push((core_idx, t));
                    }
                }
            }
        } else if let Some(rest) = line.strip_prefix("btime ") {
            btime = rest.trim().parse::<u64>().unwrap_or(0);
        }
    }

    cores.sort_by_key(|(idx, _)| *idx);
    let core_ticks = cores.into_iter().map(|(_, t)| t).collect();
    (global, core_ticks, btime)
}

pub fn read_cpu_ticks() -> (CpuTicks, Vec<CpuTicks>) {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        let (global, cores, _) = parse_proc_stat(&content);
        (global, cores)
    } else {
        (CpuTicks::default(), Vec::new())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemInfo {
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub buffers: u64,
    pub cached: u64,
    pub sreclaimable: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}

impl MemInfo {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    pub fn cached_bytes(&self) -> u64 {
        let combined = self
            .buffers
            .saturating_add(self.cached)
            .saturating_add(self.sreclaimable);
        if combined > 0 {
            combined
        } else {
            self.available.saturating_sub(self.free)
        }
    }

    pub fn swap_used(&self) -> u64 {
        self.swap_total.saturating_sub(self.swap_free)
    }
}

pub fn parse_meminfo(content: &str) -> MemInfo {
    let mut info = MemInfo::default();
    let mut has_available = false;

    for line in content.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let kb = rest
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let bytes = kb.saturating_mul(1024);

        match key {
            "MemTotal" => info.total = bytes,
            "MemFree" => info.free = bytes,
            "MemAvailable" => {
                info.available = bytes;
                has_available = true;
            }
            "Buffers" => info.buffers = bytes,
            "Cached" => info.cached = bytes,
            "SReclaimable" => info.sreclaimable = bytes,
            "SwapTotal" => info.swap_total = bytes,
            "SwapFree" => info.swap_free = bytes,
            _ => {}
        }
    }

    if !has_available {
        info.available = info
            .free
            .saturating_add(info.buffers)
            .saturating_add(info.cached)
            .saturating_add(info.sreclaimable)
            .min(info.total);
    }

    info
}

pub fn read_meminfo() -> MemInfo {
    fs::read_to_string("/proc/meminfo")
        .map(|s| parse_meminfo(&s))
        .unwrap_or_default()
}

pub fn parse_load_average(content: &str) -> (f64, f64, f64) {
    let mut parts = content.split_whitespace();
    let one = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let five = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let fifteen = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    (one, five, fifteen)
}

pub fn read_load_average() -> (f64, f64, f64) {
    fs::read_to_string("/proc/loadavg")
        .map(|s| parse_load_average(&s))
        .unwrap_or((0.0, 0.0, 0.0))
}

pub fn parse_uptime(content: &str) -> u64 {
    content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(0)
}

pub fn read_uptime_and_boot_time() -> (u64, u64) {
    let uptime = fs::read_to_string("/proc/uptime")
        .map(|s| parse_uptime(&s))
        .unwrap_or(0);
    let boot_time = fs::read_to_string("/proc/stat")
        .map(|s| parse_proc_stat(&s).2)
        .unwrap_or(0);
    (uptime, boot_time)
}

pub fn parse_os_release(content: &str) -> Option<String> {
    let mut pretty = None;
    let mut name = None;
    let mut version = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
            match k {
                "PRETTY_NAME" if !val.is_empty() => pretty = Some(val),
                "NAME" if !val.is_empty() => name = Some(val),
                "VERSION" if !val.is_empty() => version = Some(val),
                _ => {}
            }
        }
    }

    pretty.or_else(|| match (name, version) {
        (Some(n), Some(v)) => Some(format!("{} {}", n, v)),
        (Some(n), None) => Some(n),
        _ => None,
    })
}

pub fn read_os_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| parse_os_release(&s))
        .or_else(|| {
            fs::read_to_string("/usr/lib/os-release")
                .ok()
                .and_then(|s| parse_os_release(&s))
        })
        .unwrap_or_else(|| "Linux".to_string())
}

pub fn read_kernel_version() -> String {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
}

pub fn read_hostname() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

#[derive(Clone, Debug, Default)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub family: String,
    pub cache_size: String,
    pub bogomips: String,
    pub virtualization: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub core_mhz: Vec<u64>,
}

pub fn parse_cpuinfo(content: &str) -> CpuInfo {
    let mut brand = String::new();
    let mut vendor = "N/A".to_string();
    let mut family = "N/A".to_string();
    let mut cache_size = "N/A".to_string();
    let mut bogomips = "N/A".to_string();
    let mut virtualization = "Disabled/None".to_string();
    let mut logical_cores = 0usize;
    let mut core_mhz = Vec::new();

    let mut physical_pairs: HashSet<(u32, u32)> = HashSet::new();
    let mut cur_phys_id: Option<u32> = None;
    let mut cur_core_id: Option<u32> = None;
    let mut cpu_cores_hint: Option<usize> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            if let (Some(p), Some(c)) = (cur_phys_id.take(), cur_core_id.take()) {
                physical_pairs.insert((p, c));
            }
            continue;
        }
        let Some((raw_k, raw_v)) = line.split_once(':') else {
            continue;
        };
        let k = raw_k.trim();
        let v = raw_v.trim();

        match k {
            "processor" => {
                logical_cores += 1;
            }
            "model name" | "Model" | "Hardware" if brand.is_empty() => {
                brand = v.to_string();
            }
            "vendor_id" if vendor == "N/A" => {
                vendor = v.to_string();
            }
            "cpu family" if family == "N/A" => {
                family = v.to_string();
            }
            "cache size" if cache_size == "N/A" => {
                cache_size = v.to_string();
            }
            "bogomips" | "BogoMIPS" if bogomips == "N/A" => {
                bogomips = v.to_string();
            }
            "cpu MHz" => {
                let mhz = v.parse::<f64>().map(|f| f.round() as u64).unwrap_or(0);
                core_mhz.push(mhz);
            }
            "physical id" => {
                cur_phys_id = v.parse::<u32>().ok();
            }
            "core id" => {
                cur_core_id = v.parse::<u32>().ok();
            }
            "cpu cores" if cpu_cores_hint.is_none() => {
                cpu_cores_hint = v.parse::<usize>().ok();
            }
            "flags" | "Features" if virtualization == "Disabled/None" => {
                if v.split_whitespace().any(|f| f == "vmx") {
                    virtualization = "Intel VT-x".to_string();
                } else if v.split_whitespace().any(|f| f == "svm") {
                    virtualization = "AMD-V".to_string();
                }
            }
            _ => {}
        }
    }

    if let (Some(p), Some(c)) = (cur_phys_id, cur_core_id) {
        physical_pairs.insert((p, c));
    }

    let physical_cores = if !physical_pairs.is_empty() {
        physical_pairs.len()
    } else if let Some(hint) = cpu_cores_hint {
        hint
    } else {
        logical_cores
    };

    if brand.is_empty() {
        brand = "N/A".to_string();
    }

    CpuInfo {
        brand,
        vendor,
        family,
        cache_size,
        bogomips,
        virtualization,
        logical_cores,
        physical_cores,
        core_mhz,
    }
}

pub fn read_cpu_info() -> CpuInfo {
    fs::read_to_string("/proc/cpuinfo")
        .map(|s| parse_cpuinfo(&s))
        .unwrap_or_default()
}

pub fn read_core_freq_mhz(core_id: usize, fallback_mhz: u64) -> u64 {
    let path = format!(
        "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
        core_id
    );
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|khz| khz / 1000)
        .unwrap_or(fallback_mhz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proc_stat_and_usage_delta() {
        let stat1 = "\
cpu  100 10 50 800 40 0 0 0 0 0
cpu0 50 5 25 400 20 0 0 0 0 0
cpu1 50 5 25 400 20 0 0 0 0 0
btime 1700000000
";
        let stat2 = "\
cpu  150 10 75 825 40 0 0 0 0 0
cpu0 100 5 25 420 20 0 0 0 0 0
cpu1 50 5 50 405 20 0 0 0 0 0
btime 1700000000
";
        let (g1, c1, b1) = parse_proc_stat(stat1);
        let (g2, c2, b2) = parse_proc_stat(stat2);
        assert_eq!(b1, 1700000000);
        assert_eq!(b2, 1700000000);
        assert_eq!(c1.len(), 2);
        assert_eq!(c2.len(), 2);

        // total_delta = 100, idle_delta = 25 -> 75.0%
        let global_pct = g2.usage_percent(&g1);
        assert!((global_pct - 75.0).abs() < 0.01);
        // core0: total_delta = 70, active = 50 -> 71.43%
        let c0_pct = c2[0].usage_percent(&c1[0]);
        assert!((c0_pct - 71.428).abs() < 0.1);
    }

    #[test]
    fn test_parse_meminfo_values() {
        let content = "\
MemTotal:       16384000 kB
MemFree:         2048000 kB
MemAvailable:    8192000 kB
Buffers:          512000 kB
Cached:          4096000 kB
SReclaimable:     512000 kB
SwapTotal:       4096000 kB
SwapFree:        3072000 kB
";
        let mem = parse_meminfo(content);
        assert_eq!(mem.total, 16384000 * 1024);
        assert_eq!(mem.used(), (16384000 - 8192000) * 1024);
        assert_eq!(mem.cached_bytes(), (512000 + 4096000 + 512000) * 1024);
        assert_eq!(mem.swap_used(), 1024000 * 1024);
    }

    #[test]
    fn test_parse_loadavg_uptime_os_release() {
        let (l1, l5, l15) = parse_load_average("1.25 0.85 0.42 2/512 12345");
        assert!((l1 - 1.25).abs() < 0.001);
        assert!((l5 - 0.85).abs() < 0.001);
        assert!((l15 - 0.42).abs() < 0.001);

        assert_eq!(parse_uptime("3661.84 14000.12"), 3661);

        let os_rel = "NAME=\"Ubuntu\"\nVERSION=\"24.04 LTS\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n";
        assert_eq!(parse_os_release(os_rel).as_deref(), Some("Ubuntu 24.04.1 LTS"));
    }

    #[test]
    fn test_parse_cpuinfo_topology() {
        let content = "\
processor\t: 0
vendor_id\t: AuthenticAMD
cpu family\t: 25
model name\t: AMD Ryzen 7 5800X
cpu MHz\t\t: 3800.123
cache size\t: 512 KB
physical id\t: 0
core id\t\t: 0
cpu cores\t: 2
flags\t\t: fpu vme de pse svm
bogomips\t: 7600.00

processor\t: 1
vendor_id\t: AuthenticAMD
cpu family\t: 25
model name\t: AMD Ryzen 7 5800X
cpu MHz\t\t: 4100.456
cache size\t: 512 KB
physical id\t: 0
core id\t\t: 0
cpu cores\t: 2
flags\t\t: fpu vme de pse svm
bogomips\t: 7600.00

processor\t: 2
vendor_id\t: AuthenticAMD
cpu family\t: 25
model name\t: AMD Ryzen 7 5800X
cpu MHz\t\t: 4200.000
cache size\t: 512 KB
physical id\t: 0
core id\t\t: 1
cpu cores\t: 2
flags\t\t: fpu vme de pse svm
bogomips\t: 7600.00
";
        let info = parse_cpuinfo(content);
        assert_eq!(info.brand, "AMD Ryzen 7 5800X");
        assert_eq!(info.vendor, "AuthenticAMD");
        assert_eq!(info.family, "25");
        assert_eq!(info.virtualization, "AMD-V");
        assert_eq!(info.logical_cores, 3);
        assert_eq!(info.physical_cores, 2);
        assert_eq!(info.core_mhz, vec![3800, 4100, 4200]);
    }
}
