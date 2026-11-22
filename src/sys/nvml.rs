use std::collections::HashMap;
use std::fs;

use crate::types::GpuInfo;

#[cfg(not(target_env = "musl"))]
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};

#[cfg(not(target_env = "musl"))]
const RTLD_LAZY: c_int = 1;
#[cfg(not(target_env = "musl"))]
const NVML_SUCCESS: c_uint = 0;
#[cfg(not(target_env = "musl"))]
const NVML_TEMPERATURE_GPU: c_uint = 0;
#[cfg(not(target_env = "musl"))]
const NVML_CLOCK_GRAPHICS: c_uint = 0;
#[cfg(not(target_env = "musl"))]
const NVML_CLOCK_MEM: c_uint = 2;

#[cfg(not(target_env = "musl"))]
type NvmlDevice = *mut c_void;

#[cfg(not(target_env = "musl"))]
#[repr(C)]
#[derive(Default)]
struct NvmlUtilization {
    gpu: c_uint,
    memory: c_uint,
}

#[cfg(not(target_env = "musl"))]
#[repr(C)]
#[derive(Default)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

#[cfg(not(target_env = "musl"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NvmlProcessInfoV2 {
    pid: c_uint,
    used_gpu_memory: u64,
    gpu_instance_id: c_uint,
    compute_instance_id: c_uint,
}

#[cfg(not(target_env = "musl"))]
#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

#[cfg(not(target_env = "musl"))]
pub struct NvmlClient {
    lib_handle: *mut c_void,
    shutdown_fn: unsafe extern "C" fn() -> c_uint,
    driver_ver_fn: unsafe extern "C" fn(*mut c_char, c_uint) -> c_uint,
    device_count_fn: unsafe extern "C" fn(*mut c_uint) -> c_uint,
    device_handle_fn: unsafe extern "C" fn(c_uint, *mut NvmlDevice) -> c_uint,
    device_name_fn: unsafe extern "C" fn(NvmlDevice, *mut c_char, c_uint) -> c_uint,
    util_rates_fn: unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> c_uint,
    mem_info_fn: unsafe extern "C" fn(NvmlDevice, *mut NvmlMemory) -> c_uint,
    temp_fn: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> c_uint,
    power_usage_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    power_limit_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    clock_info_fn: Option<unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> c_uint>,
    fan_speed_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    pcie_curr_gen_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    pcie_curr_width_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    pcie_max_gen_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    pcie_max_width_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_uint>,
    throttle_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut u64) -> c_uint>,
    compute_procs_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut NvmlProcessInfoV2) -> c_uint>,
    graphics_procs_fn: Option<unsafe extern "C" fn(NvmlDevice, *mut c_uint, *mut NvmlProcessInfoV2) -> c_uint>,
}

#[cfg(not(target_env = "musl"))]
unsafe impl Send for NvmlClient {}

#[cfg(not(target_env = "musl"))]
unsafe fn load_sym<T: Copy>(handle: *mut c_void, name: &str) -> Option<T> {
    let c_name = CString::new(name).ok()?;
    let ptr = dlsym(handle, c_name.as_ptr());
    if ptr.is_null() {
        None
    } else {
        Some(std::mem::transmute_copy(&ptr))
    }
}

#[cfg(not(target_env = "musl"))]
impl NvmlClient {
    pub fn open() -> Option<Self> {
        let mut handle = std::ptr::null_mut();
        for lib in ["libnvidia-ml.so.1", "libnvidia-ml.so"] {
            if let Ok(c_lib) = CString::new(lib) {
                handle = unsafe { dlopen(c_lib.as_ptr(), RTLD_LAZY) };
                if !handle.is_null() {
                    break;
                }
            }
        }
        if handle.is_null() {
            return None;
        }

        unsafe {
            let init_fn: unsafe extern "C" fn() -> c_uint = match load_sym(handle, "nvmlInit_v2") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            if init_fn() != NVML_SUCCESS {
                dlclose(handle);
                return None;
            }

            let shutdown_fn = match load_sym(handle, "nvmlShutdown") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let driver_ver_fn = match load_sym(handle, "nvmlSystemGetDriverVersion") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let device_count_fn = match load_sym(handle, "nvmlDeviceGetCount_v2") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let device_handle_fn = match load_sym(handle, "nvmlDeviceGetHandleByIndex_v2") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let device_name_fn = match load_sym(handle, "nvmlDeviceGetName") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let util_rates_fn = match load_sym(handle, "nvmlDeviceGetUtilizationRates") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let mem_info_fn = match load_sym(handle, "nvmlDeviceGetMemoryInfo") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };
            let temp_fn = match load_sym(handle, "nvmlDeviceGetTemperature") {
                Some(f) => f,
                None => {
                    dlclose(handle);
                    return None;
                }
            };

            Some(Self {
                lib_handle: handle,
                shutdown_fn,
                driver_ver_fn,
                device_count_fn,
                device_handle_fn,
                device_name_fn,
                util_rates_fn,
                mem_info_fn,
                temp_fn,
                power_usage_fn: load_sym(handle, "nvmlDeviceGetPowerUsage"),
                power_limit_fn: load_sym(handle, "nvmlDeviceGetPowerManagementLimit"),
                clock_info_fn: load_sym(handle, "nvmlDeviceGetClockInfo"),
                fan_speed_fn: load_sym(handle, "nvmlDeviceGetFanSpeed"),
                pcie_curr_gen_fn: load_sym(handle, "nvmlDeviceGetCurrPcieLinkGeneration"),
                pcie_curr_width_fn: load_sym(handle, "nvmlDeviceGetCurrPcieLinkWidth"),
                pcie_max_gen_fn: load_sym(handle, "nvmlDeviceGetMaxPcieLinkGeneration"),
                pcie_max_width_fn: load_sym(handle, "nvmlDeviceGetMaxPcieLinkWidth"),
                throttle_fn: load_sym(handle, "nvmlDeviceGetCurrentClocksThrottleReasons"),
                compute_procs_fn: load_sym(handle, "nvmlDeviceGetComputeRunningProcesses_v3"),
                graphics_procs_fn: load_sym(handle, "nvmlDeviceGetGraphicsRunningProcesses_v3"),
            })
        }
    }

    pub fn query_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        let mut count: c_uint = 0;
        if unsafe { (self.device_count_fn)(&mut count) } != NVML_SUCCESS {
            return Err("nvmlDeviceGetCount_v2 failed".to_string());
        }

        let mut ver_buf = [0 as c_char; 80];
        let driver_version = if unsafe { (self.driver_ver_fn)(ver_buf.as_mut_ptr(), ver_buf.len() as c_uint) } == NVML_SUCCESS {
            unsafe { CStr::from_ptr(ver_buf.as_ptr()).to_string_lossy().into_owned() }
        } else {
            "Unknown".to_string()
        };

        let mut gpus = Vec::with_capacity(count as usize);

        for i in 0..count {
            let mut dev: NvmlDevice = std::ptr::null_mut();
            if unsafe { (self.device_handle_fn)(i, &mut dev) } != NVML_SUCCESS || dev.is_null() {
                continue;
            }

            let mut name_buf = [0 as c_char; 96];
            let name = if unsafe { (self.device_name_fn)(dev, name_buf.as_mut_ptr(), name_buf.len() as c_uint) } == NVML_SUCCESS {
                unsafe { CStr::from_ptr(name_buf.as_ptr()).to_string_lossy().into_owned() }
            } else {
                format!("NVIDIA GPU {}", i)
            };

            let mut util = NvmlUtilization::default();
            let (utilization, memory_utilization) = if unsafe { (self.util_rates_fn)(dev, &mut util) } == NVML_SUCCESS {
                (util.gpu, Some(util.memory))
            } else {
                (0, None)
            };

            let mut mem = NvmlMemory::default();
            let (memory_used, memory_total) = if unsafe { (self.mem_info_fn)(dev, &mut mem) } == NVML_SUCCESS {
                (mem.used, mem.total)
            } else {
                (0, 0)
            };

            let mut temp: c_uint = 0;
            let temperature = if unsafe { (self.temp_fn)(dev, NVML_TEMPERATURE_GPU, &mut temp) } == NVML_SUCCESS {
                temp
            } else {
                0
            };

            let power_usage = self.power_usage_fn.and_then(|f| {
                let mut mw: c_uint = 0;
                if unsafe { f(dev, &mut mw) } == NVML_SUCCESS { Some(mw) } else { None }
            }).unwrap_or(0);

            let power_limit = self.power_limit_fn.and_then(|f| {
                let mut mw: c_uint = 0;
                if unsafe { f(dev, &mut mw) } == NVML_SUCCESS { Some(mw) } else { None }
            });

            let graphics_clock = self.clock_info_fn.and_then(|f| {
                let mut mhz: c_uint = 0;
                if unsafe { f(dev, NVML_CLOCK_GRAPHICS, &mut mhz) } == NVML_SUCCESS { Some(mhz) } else { None }
            }).unwrap_or(0);

            let memory_clock = self.clock_info_fn.and_then(|f| {
                let mut mhz: c_uint = 0;
                if unsafe { f(dev, NVML_CLOCK_MEM, &mut mhz) } == NVML_SUCCESS { Some(mhz) } else { None }
            }).unwrap_or(0);

            let fan_speed = self.fan_speed_fn.and_then(|f| {
                let mut pct: c_uint = 0;
                if unsafe { f(dev, &mut pct) } == NVML_SUCCESS { Some(pct) } else { None }
            });

            let pci_link_gen = self.pcie_curr_gen_fn.and_then(|f| {
                let mut g: c_uint = 0;
                if unsafe { f(dev, &mut g) } == NVML_SUCCESS { Some(g) } else { None }
            });
            let pci_link_width = self.pcie_curr_width_fn.and_then(|f| {
                let mut w: c_uint = 0;
                if unsafe { f(dev, &mut w) } == NVML_SUCCESS { Some(w) } else { None }
            });
            let pci_link_gen_max = self.pcie_max_gen_fn.and_then(|f| {
                let mut g: c_uint = 0;
                if unsafe { f(dev, &mut g) } == NVML_SUCCESS { Some(g) } else { None }
            });
            let pci_link_width_max = self.pcie_max_width_fn.and_then(|f| {
                let mut w: c_uint = 0;
                if unsafe { f(dev, &mut w) } == NVML_SUCCESS { Some(w) } else { None }
            });

            let (throttle_reasons, is_throttling) = self.throttle_fn.map(|f| {
                let mut mask: u64 = 0;
                if unsafe { f(dev, &mut mask) } == NVML_SUCCESS {
                    crate::monitors::gpu_monitor::parse_nvidia_throttle_mask(mask)
                } else {
                    (None, false)
                }
            }).unwrap_or((None, false));

            let gpu_processes = self.query_device_processes(dev);

            gpus.push(GpuInfo {
                name,
                brand: "NVIDIA".to_string(),
                utilization,
                memory_utilization,
                memory_used,
                memory_total,
                temperature,
                memory_temperature: None,
                vram_temp: None,
                power_usage,
                graphics_clock,
                memory_clock,
                fan_speed,
                fan_rpm: None,
                utilization_history: Vec::new(),
                memory_history: Vec::new(),
                pci_link_gen,
                pci_link_width,
                pci_link_gen_max,
                pci_link_width_max,
                power_limit,
                throttle_reasons,
                is_throttling,
                driver_version: driver_version.clone(),
                gpu_processes,
            });
        }

        Ok(gpus)
    }

    fn query_device_processes(&self, dev: NvmlDevice) -> Vec<(u32, u64)> {
        let mut by_pid: HashMap<u32, u64> = HashMap::new();
        for func in [self.compute_procs_fn, self.graphics_procs_fn].into_iter().flatten() {
            let mut count: c_uint = 32;
            let mut buf = [NvmlProcessInfoV2::default(); 32];
            if unsafe { func(dev, &mut count, buf.as_mut_ptr()) } == NVML_SUCCESS {
                for entry in buf.iter().take((count as usize).min(buf.len())) {
                    // NVML returns u64::MAX when usedGpuMemory is unavailable (e.g. WDDM/vGPU)
                    let mem = if entry.used_gpu_memory == u64::MAX { 0 } else { entry.used_gpu_memory };
                    let slot = by_pid.entry(entry.pid).or_insert(0);
                    if mem > *slot {
                        *slot = mem;
                    }
                }
            }
        }
        let mut list: Vec<(u32, u64)> = by_pid.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }
}

#[cfg(not(target_env = "musl"))]
impl Drop for NvmlClient {
    fn drop(&mut self) {
        unsafe {
            (self.shutdown_fn)();
            dlclose(self.lib_handle);
        }
    }
}

#[cfg(target_env = "musl")]
pub struct NvmlClient;

#[cfg(target_env = "musl")]
impl NvmlClient {
    pub fn open() -> Option<Self> {
        None
    }

    pub fn query_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        Err("NVML dlopen unavailable on static musl".to_string())
    }
}

fn parse_size_to_bytes(val_str: &str) -> Option<u64> {
    let mut parts = val_str.split_whitespace();
    let num = parts.next()?.parse::<u64>().ok()?;
    let unit = parts.next().unwrap_or("B");
    let mult = match unit {
        "KiB" | "KB" | "kB" => 1024,
        "MiB" | "MB" => 1024 * 1024,
        "GiB" | "GB" => 1024 * 1024 * 1024,
        _ => 1,
    };
    Some(num.saturating_mul(mult))
}

pub fn parse_drm_fdinfo_vram(content: &str) -> Option<(String, u64)> {
    let mut driver = None;
    let mut vram_bytes = 0u64;

    for line in content.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();
        match key {
            "drm-driver" => driver = Some(val.to_string()),
            "drm-memory-vram" | "drm-resident-vram" | "drm-total-vram" => {
                if let Some(bytes) = parse_size_to_bytes(val) {
                    if bytes > vram_bytes {
                        vram_bytes = bytes;
                    }
                }
            }
            _ => {}
        }
    }

    driver.map(|d| (d, vram_bytes))
}

pub fn scan_drm_gpu_processes(driver_filter: &str) -> Vec<(u32, u64)> {
    let mut by_pid: HashMap<u32, u64> = HashMap::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };

        let fdinfo_dir = entry.path().join("fdinfo");
        let Ok(fd_entries) = fs::read_dir(fdinfo_dir) else {
            continue;
        };

        for fd_entry in fd_entries.flatten() {
            if let Ok(content) = fs::read_to_string(fd_entry.path()) {
                if !content.contains("drm-driver:") {
                    continue;
                }
                if let Some((drv, vram)) = parse_drm_fdinfo_vram(&content) {
                    if drv.eq_ignore_ascii_case(driver_filter) && vram > 0 {
                        let slot = by_pid.entry(pid).or_insert(0);
                        if vram > *slot {
                            *slot = vram;
                        }
                    }
                }
            }
        }
    }

    let mut list: Vec<(u32, u64)> = by_pid.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_drm_fdinfo_amd_and_intel() {
        let amd_fdinfo = "\
pos:\t0
flags:\t02100002
drm-driver:\tamdgpu
drm-client-id:\t42
drm-memory-vram:\t1536 MiB
drm-memory-gtt:\t256 MiB
";
        let (drv, vram) = parse_drm_fdinfo_vram(amd_fdinfo).expect("amdgpu fdinfo parse failed");
        assert_eq!(drv, "amdgpu");
        assert_eq!(vram, 1536 * 1024 * 1024);

        let intel_fdinfo = "\
drm-driver:\txe
drm-resident-vram:\t524288 KiB
";
        let (drv2, vram2) = parse_drm_fdinfo_vram(intel_fdinfo).expect("xe fdinfo parse failed");
        assert_eq!(drv2, "xe");
        assert_eq!(vram2, 524288 * 1024);
    }

    #[test]
    fn test_nvml_open_graceful() {
        if let Some(client) = NvmlClient::open() {
            let _ = client.query_gpus();
        }
    }
}
