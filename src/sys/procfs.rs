use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::sync::OnceLock;

use crate::types::{Pid, PressureStallInfo, PsiMetric};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: String,
}

pub fn unescape_mount_path(s: &str) -> String {
    s.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

fn is_pseudo_fs(fs_type: &str, device: &str, mount_point: &str) -> bool {
    const IGNORED_FS: &[&str] = &[
        "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "cgroup", "cgroup2",
        "pstore", "bpf", "tracefs", "debugfs", "securityfs", "hugetlbfs",
        "mqueue", "fusectl", "configfs", "autofs", "nsfs", "overlay",
        "squashfs", "ramfs", "rpc_pipefs", "efivarfs", "binfmt_misc",
        "fuse.portal", "fuse.gvfsd-fuse", "fuse.lxcfs", "selinuxfs",
    ];
    if IGNORED_FS.contains(&fs_type) {
        return true;
    }
    if mount_point.starts_with("/snap/") || mount_point.starts_with("/var/lib/docker/") {
        return true;
    }
    if device.starts_with("/dev/loop") {
        return true;
    }
    false
}

pub fn parse_mounts(content: &str) -> Vec<MountEntry> {
    let mut mounts = Vec::new();
    let mut seen_mounts = HashSet::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let device = unescape_mount_path(parts[0]);
        let mount_point = unescape_mount_path(parts[1]);
        let fs_type = parts[2].to_string();
        let options = parts[3].to_string();

        if is_pseudo_fs(&fs_type, &device, &mount_point) {
            continue;
        }
        if !seen_mounts.insert(mount_point.clone()) {
            continue;
        }

        mounts.push(MountEntry {
            device,
            mount_point,
            fs_type,
            options,
        });
    }

    mounts
}

pub fn read_mounts() -> Vec<MountEntry> {
    fs::read_to_string("/proc/mounts")
        .map(|s| parse_mounts(&s))
        .unwrap_or_default()
}

#[repr(C)]
struct StatVfs {
    f_bsize: std::ffi::c_ulong,
    f_frsize: std::ffi::c_ulong,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: std::ffi::c_ulong,
    f_flag: std::ffi::c_ulong,
    f_namemax: std::ffi::c_ulong,
    __f_spare: [std::ffi::c_int; 6],
}

extern "C" {
    fn statvfs(path: *const std::ffi::c_char, buf: *mut StatVfs) -> std::ffi::c_int;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FsUsage {
    pub total: u64,
    pub free: u64,
    pub inodes_total: Option<u64>,
    pub inodes_used: Option<u64>,
}

pub fn stat_filesystem(mount_point: &str) -> Option<FsUsage> {
    let c_path = std::ffi::CString::new(mount_point).ok()?;
    let mut stat = std::mem::MaybeUninit::<StatVfs>::zeroed();
    unsafe {
        if statvfs(c_path.as_ptr(), stat.as_mut_ptr()) != 0 {
            return None;
        }
        let s = stat.assume_init();
        let block_size = if s.f_frsize > 0 {
            s.f_frsize as u64
        } else {
            s.f_bsize as u64
        };
        let total = s.f_blocks.saturating_mul(block_size);
        let free = s.f_bavail.saturating_mul(block_size);
        let (inodes_total, inodes_used) = if s.f_files > 0 {
            (
                Some(s.f_files),
                Some(s.f_files.saturating_sub(s.f_ffree)),
            )
        } else {
            (None, None)
        };
        Some(FsUsage {
            total,
            free,
            inodes_total,
            inodes_used,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetDevCounters {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
}

pub fn parse_net_dev(content: &str) -> Vec<NetDevCounters> {
    let mut devs = Vec::new();
    for line in content.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        let name = iface.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let cols: Vec<&str> = rest.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        devs.push(NetDevCounters {
            name,
            rx_bytes: cols[0].parse().unwrap_or(0),
            rx_packets: cols[1].parse().unwrap_or(0),
            tx_bytes: cols[8].parse().unwrap_or(0),
            tx_packets: cols[9].parse().unwrap_or(0),
        });
    }
    devs
}

pub fn read_net_dev() -> Vec<NetDevCounters> {
    fs::read_to_string("/proc/net/dev")
        .map(|s| parse_net_dev(&s))
        .unwrap_or_default()
}

extern "C" {
    fn sysconf(name: std::ffi::c_int) -> std::ffi::c_long;
}

pub fn clk_tck() -> u64 {
    static TCK: OnceLock<u64> = OnceLock::new();
    *TCK.get_or_init(|| {
        // _SC_CLK_TCK = 2 on Linux
        let v = unsafe { sysconf(2) };
        if v > 0 { v as u64 } else { 100 }
    })
}

pub fn page_size() -> u64 {
    static PAGE: OnceLock<u64> = OnceLock::new();
    *PAGE.get_or_init(|| {
        // _SC_PAGESIZE = 30 on Linux
        let v = unsafe { sysconf(30) };
        if v > 0 { v as u64 } else { 4096 }
    })
}

#[derive(Clone, Debug)]
pub struct ProcPidStat {
    pub comm: String,
    pub state: char,
    pub ppid: u32,
    pub total_ticks: u64,
    pub nice: i32,
    pub num_threads: u32,
    pub starttime_ticks: u64,
    pub vsize_bytes: u64,
    pub rss_bytes: u64,
}

pub fn proc_state_name(state: char) -> &'static str {
    match state {
        'R' => "Running",
        'S' => "Sleeping",
        'D' => "Disk Sleep",
        'Z' => "Zombie",
        'T' => "Stopped",
        't' => "Tracing Stop",
        'X' | 'x' => "Dead",
        'K' => "Wakekill",
        'W' => "Waking",
        'P' => "Parked",
        'I' => "Idle",
        _ => "Unknown",
    }
}

pub fn parse_proc_pid_stat(content: &str, page_sz: u64) -> Option<ProcPidStat> {
    let open = content.find('(')?;
    let close = content.rfind(')')?;
    if close <= open {
        return None;
    }

    let comm = content[open + 1..close].to_string();
    let rest = content.get(close + 1..)?.trim_start();
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 22 {
        return None;
    }

    let state = fields[0].chars().next().unwrap_or('?');
    let ppid = fields[1].parse::<u32>().unwrap_or(0);
    let utime = fields[11].parse::<u64>().unwrap_or(0);
    let stime = fields[12].parse::<u64>().unwrap_or(0);
    let nice = fields[16].parse::<i32>().unwrap_or(0);
    let num_threads = fields[17].parse::<u32>().unwrap_or(1);
    let starttime_ticks = fields[19].parse::<u64>().unwrap_or(0);
    let vsize_bytes = fields[20].parse::<u64>().unwrap_or(0);
    let rss_pages = fields[21].parse::<u64>().unwrap_or(0);

    Some(ProcPidStat {
        comm,
        state,
        ppid,
        total_ticks: utime.saturating_add(stime),
        nice,
        num_threads,
        starttime_ticks,
        vsize_bytes,
        rss_bytes: rss_pages.saturating_mul(page_sz),
    })
}

pub fn read_process_io(pid: Pid) -> (u64, u64, u64, u64) {
    let path = format!("/proc/{}/io", pid);
    if let Ok(content) = fs::read_to_string(path) {
        let mut read_bytes = 0;
        let mut write_bytes = 0;
        let mut rchar = 0;
        let mut wchar = 0;
        for line in content.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let v = v.trim().parse::<u64>().unwrap_or(0);
                match k.trim() {
                    "read_bytes" => read_bytes = v,
                    "write_bytes" => write_bytes = v,
                    "rchar" => rchar = v,
                    "wchar" => wchar = v,
                    _ => {}
                }
            }
        }
        (read_bytes, write_bytes, rchar, wchar)
    } else {
        (0, 0, 0, 0)
    }
}

#[derive(Clone, Debug)]
pub struct ScannedProcess {
    pub pid: Pid,
    pub uid: u32,
    pub stat: ProcPidStat,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
}

pub fn scan_processes() -> Vec<ScannedProcess> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return result;
    };
    let page_sz = page_size();

    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid_num) = name.parse::<u32>() else {
            continue;
        };
        let pid = Pid::from(pid_num);
        let proc_dir = entry.path();

        let Ok(stat_str) = fs::read_to_string(proc_dir.join("stat")) else {
            continue;
        };
        let Some(stat) = parse_proc_pid_stat(&stat_str, page_sz) else {
            continue;
        };
        let uid = fs::metadata(&proc_dir).map(|m| m.uid()).unwrap_or(0);
        let (io_read_bytes, io_write_bytes, _, _) = read_process_io(pid);

        result.push(ScannedProcess {
            pid,
            uid,
            stat,
            io_read_bytes,
            io_write_bytes,
        });
    }

    result
}

pub fn read_process_single(pid: Pid) -> Option<(u32, ProcPidStat, String, Option<String>, Vec<String>)> {
    let proc_dir = format!("/proc/{}", pid);
    let stat_str = fs::read_to_string(format!("{}/stat", proc_dir)).ok()?;
    let stat = parse_proc_pid_stat(&stat_str, page_size())?;
    let uid = fs::metadata(&proc_dir).map(|m| m.uid()).unwrap_or(0);

    let command = fs::read(format!("{}/cmdline", proc_dir))
        .ok()
        .map(|bytes| {
            bytes
                .split(|&b| b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect::<Vec<String>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| stat.comm.clone());

    let cwd = fs::read_link(format!("{}/cwd", proc_dir))
        .ok()
        .map(|p| p.to_string_lossy().into_owned());

    let mut environ: Vec<String> = fs::read(format!("{}/environ", proc_dir))
        .ok()
        .map(|bytes| {
            bytes
                .split(|&b| b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect()
        })
        .unwrap_or_default();
    environ.sort();

    Some((uid, stat, command, cwd, environ))
}

pub fn parse_psi_file(content: &str) -> PsiMetric {
    let mut m = PsiMetric::default();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        let mut avg10 = 0.0f32;
        let mut avg60 = 0.0f32;
        let mut avg300 = 0.0f32;
        for token in parts {
            if let Some((k, v)) = token.split_once('=') {
                match k {
                    "avg10" => avg10 = v.parse().unwrap_or(0.0),
                    "avg60" => avg60 = v.parse().unwrap_or(0.0),
                    "avg300" => avg300 = v.parse().unwrap_or(0.0),
                    _ => {}
                }
            }
        }
        match kind {
            "some" => {
                m.some_avg10 = avg10;
                m.some_avg60 = avg60;
                m.some_avg300 = avg300;
            }
            "full" => {
                m.full_avg10 = avg10;
                m.full_avg60 = avg60;
            }
            _ => {}
        }
    }
    m
}

pub fn read_pressure_stall_info() -> PressureStallInfo {
    let cpu_str = fs::read_to_string("/proc/pressure/cpu").ok();
    let mem_str = fs::read_to_string("/proc/pressure/memory").ok();
    let io_str = fs::read_to_string("/proc/pressure/io").ok();

    let available = cpu_str.is_some() || mem_str.is_some() || io_str.is_some();
    PressureStallInfo {
        available,
        cpu: cpu_str.as_deref().map(parse_psi_file).unwrap_or_default(),
        memory: mem_str.as_deref().map(parse_psi_file).unwrap_or_default(),
        io: io_str.as_deref().map(parse_psi_file).unwrap_or_default(),
    }
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

    #[test]
    fn test_parse_mounts_and_net_dev() {
        let mounts_txt = "\
proc /proc proc rw,nosuid 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
/dev/sda1 /mnt/My\\040Drive xfs rw 0 0
/dev/loop0 /snap/core/1 squashfs ro 0 0
tmpfs /run tmpfs rw 0 0
";
        let mounts = parse_mounts(mounts_txt);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].device, "/dev/nvme0n1p2");
        assert_eq!(mounts[0].mount_point, "/");
        assert_eq!(mounts[0].fs_type, "ext4");
        assert_eq!(mounts[1].mount_point, "/mnt/My Drive");

        let net_txt = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo:  102400     500    0    0    0     0          0         0   102400     500    0    0    0     0       0          0
  eth0: 9876543   12345    0    0    0     0          0         0  5432100    6789    0    0    0     0       0          0
";
        let devs = parse_net_dev(net_txt);
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0].name, "lo");
        assert_eq!(devs[1].name, "eth0");
        assert_eq!(devs[1].rx_bytes, 9876543);
        assert_eq!(devs[1].rx_packets, 12345);
        assert_eq!(devs[1].tx_bytes, 5432100);
        assert_eq!(devs[1].tx_packets, 6789);
    }

    #[test]
    fn test_stat_filesystem_root() {
        let usage = stat_filesystem("/").expect("statvfs / should succeed");
        assert!(usage.total > 0);
        assert!(usage.free <= usage.total);
    }

    #[test]
    fn test_parse_proc_pid_stat_with_parens_and_spaces() {
        let line = "4242 (Web (Content) Worker) S 100 4242 100 0 -1 4194304 100 0 0 0 150 50 0 0 20 -5 8 0 98765 536870912 4096 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
        let st = parse_proc_pid_stat(line, 4096).expect("parse stat failed");
        assert_eq!(st.comm, "Web (Content) Worker");
        assert_eq!(st.state, 'S');
        assert_eq!(st.ppid, 100);
        assert_eq!(st.total_ticks, 200);
        assert_eq!(st.nice, -5);
        assert_eq!(st.num_threads, 8);
        assert_eq!(st.starttime_ticks, 98765);
        assert_eq!(st.vsize_bytes, 536870912);
        assert_eq!(st.rss_bytes, 4096 * 4096);
    }

    #[test]
    fn test_scan_self_process() {
        let self_pid = Pid::from(std::process::id());
        let scanned = scan_processes();
        assert!(scanned.iter().any(|p| p.pid == self_pid));
        let detail = read_process_single(self_pid).expect("read self process failed");
        assert!(!detail.1.comm.is_empty());
        assert!(detail.1.rss_bytes > 0);
    }

    #[test]
    fn test_parse_psi_file() {
        let mem_psi = "\
some avg10=12.34 avg60=5.67 avg300=1.23 total=987654
full avg10=3.45 avg60=1.12 avg300=0.45 total=123456
";
        let m = parse_psi_file(mem_psi);
        assert!((m.some_avg10 - 12.34).abs() < 0.01);
        assert!((m.some_avg60 - 5.67).abs() < 0.01);
        assert!((m.some_avg300 - 1.23).abs() < 0.01);
        assert!((m.full_avg10 - 3.45).abs() < 0.01);
        assert!((m.full_avg60 - 1.12).abs() < 0.01);

        let cpu_psi = "some avg10=0.80 avg60=0.40 avg300=0.10 total=42000\n";
        let c = parse_psi_file(cpu_psi);
        assert!((c.some_avg10 - 0.80).abs() < 0.01);
        assert_eq!(c.full_avg10, 0.0);
    }
}
