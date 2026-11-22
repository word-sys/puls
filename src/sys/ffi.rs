use std::ffi::{c_char, c_int, c_long, c_void};
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::types::{Pid, UserSessionInfo};
use crate::utils::format_timestamp;

const PRIO_PROCESS: i32 = 0;
const UTMP_RECORD_SIZE: usize = 384;
const UTMP_USER_PROCESS: i16 = 7;
const CPU_SET_WORDS: usize = 16; // 1024 logical CPUs
const IOPRIO_WHO_PROCESS: c_int = 1;

#[cfg(target_arch = "x86_64")]
const SYS_IOPRIO_SET: c_long = 251;
#[cfg(target_arch = "x86_64")]
const SYS_IOPRIO_GET: c_long = 252;

#[cfg(target_arch = "aarch64")]
const SYS_IOPRIO_SET: c_long = 30;
#[cfg(target_arch = "aarch64")]
const SYS_IOPRIO_GET: c_long = 31;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const SYS_IOPRIO_SET: c_long = -1;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const SYS_IOPRIO_GET: c_long = -1;

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn setpriority(which: i32, who: u32, prio: i32) -> i32;
    fn sethostname(name: *const c_char, len: usize) -> i32;
    fn sched_getaffinity(pid: i32, cpusetsize: usize, mask: *mut c_void) -> i32;
    fn sched_setaffinity(pid: i32, cpusetsize: usize, mask: *const c_void) -> i32;
    fn syscall(num: c_long, ...) -> c_long;
}

pub fn get_cpu_affinity(pid: Pid) -> Result<Vec<usize>, String> {
    let mut mask = [0u64; CPU_SET_WORDS];
    let ret = unsafe {
        sched_getaffinity(
            pid.as_u32() as i32,
            std::mem::size_of_val(&mask),
            mask.as_mut_ptr() as *mut c_void,
        )
    };
    if ret != 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(affinity_mask_to_cpus(&mask))
}

pub fn set_cpu_affinity(pid: Pid, cpus: &[usize]) -> Result<(), String> {
    if cpus.is_empty() {
        return Err("At least one CPU core must be selected".to_string());
    }
    let mask = cpus_to_affinity_mask(cpus);
    let ret = unsafe {
        sched_setaffinity(
            pid.as_u32() as i32,
            std::mem::size_of_val(&mask),
            mask.as_ptr() as *const c_void,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

pub fn affinity_mask_to_cpus(mask: &[u64; CPU_SET_WORDS]) -> Vec<usize> {
    let mut cpus = Vec::new();
    for (word_idx, &word) in mask.iter().enumerate() {
        if word == 0 {
            continue;
        }
        for bit in 0..64 {
            if (word & (1u64 << bit)) != 0 {
                cpus.push(word_idx * 64 + bit);
            }
        }
    }
    cpus
}

pub fn cpus_to_affinity_mask(cpus: &[usize]) -> [u64; CPU_SET_WORDS] {
    let mut mask = [0u64; CPU_SET_WORDS];
    for &cpu in cpus {
        let word_idx = cpu / 64;
        let bit = cpu % 64;
        if word_idx < CPU_SET_WORDS {
            mask[word_idx] |= 1u64 << bit;
        }
    }
    mask
}

pub fn format_cpu_list(cpus: &[usize]) -> String {
    if cpus.is_empty() {
        return "none".to_string();
    }
    let mut sorted = cpus.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut ranges = Vec::new();
    let mut start = sorted[0];
    let mut prev = sorted[0];

    for &c in &sorted[1..] {
        if c == prev + 1 {
            prev = c;
        } else {
            if start == prev {
                ranges.push(start.to_string());
            } else {
                ranges.push(format!("{}-{}", start, prev));
            }
            start = c;
            prev = c;
        }
    }
    if start == prev {
        ranges.push(start.to_string());
    } else {
        ranges.push(format!("{}-{}", start, prev));
    }
    ranges.join(",")
}

pub fn get_io_priority(pid: Pid) -> Option<(u8, u8)> {
    if SYS_IOPRIO_GET < 0 {
        return None;
    }
    let ret = unsafe { syscall(SYS_IOPRIO_GET, IOPRIO_WHO_PROCESS, pid.as_u32() as c_int) };
    if ret < 0 {
        return None;
    }
    let val = ret as u32;
    let class = ((val >> 13) & 0x7) as u8;
    let level = (val & 0x1FFF) as u8;
    Some((class, level))
}

pub fn set_io_priority(pid: Pid, class: u8, level: u8) -> Result<(), String> {
    if SYS_IOPRIO_SET < 0 {
        return Err("ioprio_set unsupported on this architecture".to_string());
    }
    let ioprio = (((class as u32) & 0x7) << 13) | ((level.min(7) as u32) & 0x1FFF);
    let ret = unsafe {
        syscall(
            SYS_IOPRIO_SET,
            IOPRIO_WHO_PROCESS,
            pid.as_u32() as c_int,
            ioprio as c_int,
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

pub fn format_io_priority(class: u8, level: u8) -> String {
    match class {
        1 => format!("Real-Time ({})", level),
        2 => format!("Best-Effort ({})", level),
        3 => "Idle".to_string(),
        _ => format!("None ({})", level),
    }
}

pub fn send_signal(pid: Pid, signal: i32) -> Result<(), String> {
    let raw_pid = pid.as_u32() as i32;
    if raw_pid <= 0 {
        return Err(format!("Invalid PID: {}", pid));
    }
    let ret = unsafe { kill(raw_pid, signal) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

pub fn set_process_nice(pid: Pid, nice: i32) -> Result<(), String> {
    let clamped = nice.clamp(-20, 19);
    let ret = unsafe { setpriority(PRIO_PROCESS, pid.as_u32(), clamped) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().to_string())
    }
}

pub fn set_hostname_native(new_hostname: &str) -> Result<(), String> {
    let trimmed = new_hostname.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err("Invalid hostname length (must be 1..=64 bytes)".to_string());
    }
    let ret = unsafe { sethostname(trimmed.as_ptr() as *const c_char, trimmed.len()) };
    if ret != 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    atomic_write_file(Path::new("/etc/hostname"), format!("{}\n", trimmed).as_bytes())
        .map_err(|e| format!("Failed writing /etc/hostname: {}", e))?;
    Ok(())
}

pub fn read_timezone_native() -> Option<String> {
    if let Ok(target) = fs::read_link("/etc/localtime") {
        let s = target.to_string_lossy();
        if let Some((_, tz)) = s.split_once("zoneinfo/") {
            let trimmed = tz.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    if let Ok(tz) = fs::read_to_string("/etc/timezone") {
        let trimmed = tz.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

pub fn set_timezone_native(timezone: &str) -> Result<(), String> {
    let tz = timezone.trim();
    if tz.is_empty() || tz.contains("..") || tz.starts_with('/') {
        return Err(format!("Invalid timezone: {}", timezone));
    }
    let zone_path = Path::new("/usr/share/zoneinfo").join(tz);
    if !zone_path.exists() {
        return Err(format!("Timezone not found: {}", zone_path.display()));
    }

    let tmp_link = Path::new("/etc/.localtime.puls.tmp");
    let _ = fs::remove_file(tmp_link);
    symlink(&zone_path, tmp_link)
        .map_err(|e| format!("Failed creating localtime symlink: {}", e))?;
    fs::rename(tmp_link, "/etc/localtime").map_err(|e| {
        let _ = fs::remove_file(tmp_link);
        format!("Failed replacing /etc/localtime: {}", e)
    })?;

    if Path::new("/etc/timezone").exists() {
        let _ = atomic_write_file(Path::new("/etc/timezone"), format!("{}\n", tz).as_bytes());
    }
    Ok(())
}

pub fn atomic_write_file(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let fname = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let tmp_path = parent.join(format!(".{}.puls.tmp.{}", fname, std::process::id()));

    let existing_perms = fs::metadata(path).ok().map(|m| m.permissions());

    let mut file = File::create(&tmp_path)
        .map_err(|e| format!("Failed creating temp file {}: {}", tmp_path.display(), e))?;
    if let Err(e) = file.write_all(content).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("Failed writing temp file {}: {}", tmp_path.display(), e));
    }
    drop(file);

    if let Some(perms) = existing_perms {
        let _ = fs::set_permissions(&tmp_path, perms);
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed renaming temp file to {}: {}", path.display(), e)
    })
}

fn c_str_from_bytes(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

// Linux x86_64/aarch64 struct utmp layout (384 bytes):
// 0..2 ut_type, 8..40 ut_line, 44..76 ut_user, 76..332 ut_host, 340..344 ut_tv.tv_sec
pub fn parse_utmp_bytes(data: &[u8]) -> Vec<UserSessionInfo> {
    let mut sessions = Vec::new();
    for chunk in data.chunks_exact(UTMP_RECORD_SIZE) {
        let ut_type = i16::from_ne_bytes([chunk[0], chunk[1]]);
        if ut_type != UTMP_USER_PROCESS {
            continue;
        }
        let user = c_str_from_bytes(&chunk[44..76]);
        if user.is_empty() {
            continue;
        }
        let line = c_str_from_bytes(&chunk[8..40]);
        let host_raw = c_str_from_bytes(&chunk[76..332]);
        let host = if host_raw.is_empty() {
            "local".to_string()
        } else {
            host_raw
        };
        let tv_sec = i32::from_ne_bytes([chunk[340], chunk[341], chunk[342], chunk[343]]) as i64;
        let login_time = if tv_sec > 0 {
            format_timestamp(tv_sec, "%Y-%m-%d %H:%M")
        } else {
            "unknown".to_string()
        };

        sessions.push(UserSessionInfo {
            user,
            line,
            login_time,
            host,
        });
    }
    sessions
}

pub fn parse_logind_session(content: &str, mtime_epoch: i64) -> Option<UserSessionInfo> {
    let mut user = None;
    let mut tty = None;
    let mut display = None;
    let mut remote_host = None;
    let mut state = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            match k {
                "USER" => user = Some(v.to_string()),
                "TTY" => tty = Some(v.to_string()),
                "DISPLAY" => display = Some(v.to_string()),
                "REMOTE_HOST" => remote_host = Some(v.to_string()),
                "STATE" => state = Some(v.to_string()),
                _ => {}
            }
        }
    }

    if let Some(ref st) = state {
        if st == "closing" {
            return None;
        }
    }

    let user = user?;
    let line = tty.or(display).unwrap_or_else(|| "session".to_string());
    let host = remote_host.unwrap_or_else(|| "local".to_string());
    let login_time = if mtime_epoch > 0 {
        format_timestamp(mtime_epoch, "%Y-%m-%d %H:%M")
    } else {
        "unknown".to_string()
    };

    Some(UserSessionInfo {
        user,
        line,
        login_time,
        host,
    })
}

pub fn read_user_sessions_native() -> Vec<UserSessionInfo> {
    for path in ["/run/utmp", "/var/run/utmp"] {
        if let Ok(bytes) = fs::read(path) {
            let sessions = parse_utmp_bytes(&bytes);
            if !sessions.is_empty() {
                return sessions;
            }
        }
    }

    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir("/run/systemd/sessions") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                let mtime = fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                if let Some(s) = parse_logind_session(&content, mtime) {
                    sessions.push(s);
                }
            }
        }
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_utmp_binary_record() {
        let mut record = vec![0u8; UTMP_RECORD_SIZE];
        record[0..2].copy_from_slice(&UTMP_USER_PROCESS.to_ne_bytes());
        record[8..13].copy_from_slice(b"pts/2");
        record[44..49].copy_from_slice(b"alice");
        record[76..89].copy_from_slice(b"192.168.1.50\0");
        let ts: i32 = 1700000000;
        record[340..344].copy_from_slice(&ts.to_ne_bytes());

        let sessions = parse_utmp_bytes(&record);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].user, "alice");
        assert_eq!(sessions[0].line, "pts/2");
        assert_eq!(sessions[0].host, "192.168.1.50");
        assert!(!sessions[0].login_time.is_empty());
    }

    #[test]
    fn test_parse_logind_session_file() {
        let content = "# systemd-logind\nUSER=word-sys\nTTY=tty2\nSTATE=active\n";
        let s = parse_logind_session(content, 1700000000).expect("parse logind session");
        assert_eq!(s.user, "word-sys");
        assert_eq!(s.line, "tty2");
        assert_eq!(s.host, "local");
    }

    #[test]
    fn test_atomic_write_file_preserves_content() {
        let dir = std::env::temp_dir().join(format!("puls_atomic_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("grub.conf");
        fs::write(&target, b"OLD=1\n").unwrap();

        atomic_write_file(&target, b"NEW=2\n").expect("atomic write failed");
        assert_eq!(fs::read_to_string(&target).unwrap(), "NEW=2\n");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_send_signal_zero_check_self() {
        let self_pid = Pid::from(std::process::id());
        assert!(send_signal(self_pid, 0).is_ok());
    }

    #[test]
    fn test_affinity_mask_and_format() {
        let cpus = vec![0, 1, 2, 3, 6, 8, 9];
        let mask = cpus_to_affinity_mask(&cpus);
        assert_eq!(affinity_mask_to_cpus(&mask), cpus);
        assert_eq!(format_cpu_list(&cpus), "0-3,6,8-9");

        let self_pid = Pid::from(std::process::id());
        let self_cpus = get_cpu_affinity(self_pid).expect("get_cpu_affinity failed");
        assert!(!self_cpus.is_empty());
        let _ = get_io_priority(self_pid);
    }
}
