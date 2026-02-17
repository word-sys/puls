use crate::types::GpuInfo;
use std::collections::VecDeque;
use std::process::Command;
use std::path::Path;
use std::fs;

pub struct GpuMonitor {
    gpu_history: VecDeque<Vec<u32>>,
    gpu_memory_history: VecDeque<Vec<u32>>,
    last_update: std::time::Instant,
}

impl GpuMonitor {
    pub fn new() -> Self {
        Self {
            gpu_history: VecDeque::new(),
            gpu_memory_history: VecDeque::new(),
            last_update: std::time::Instant::now(),
        }
    }
    
    pub fn get_gpu_info(&mut self) -> Result<Vec<GpuInfo>, String> {
        let mut gpus = Vec::new();
        let mut errors = Vec::new();
        
        match self.get_nvidia_gpus() {
            Ok(mut nvidia_gpus) => gpus.append(&mut nvidia_gpus),
            Err(e) => errors.push(format!("NVIDIA: {}", e)),
        }
        
        match self.get_drm_gpus() {
            Ok(mut drm_gpus) => gpus.append(&mut drm_gpus),
            Err(e) => errors.push(format!("DRM: {}", e)),
        }
        
        if gpus.is_empty() {
            if errors.is_empty() {
                Err("No supported GPUs found".to_string())
            } else {
                Err(format!("No GPUs found. Errors: {}", errors.join(", ")))
            }
        } else {
            for (i, gpu) in gpus.iter_mut().enumerate() {
                gpu.utilization_history = self.gpu_history
                    .iter()
                    .filter_map(|frame| frame.get(i).cloned())
                    .collect();
                    
                gpu.memory_history = self.gpu_memory_history
                    .iter()
                    .filter_map(|frame| frame.get(i).cloned())
                    .collect();
            }
            Ok(gpus)
        }
    }
    
    fn get_nvidia_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,clocks.gr,clocks.mem,fan.speed,driver_version")
            .arg("--format=csv,noheader,nounits")
            .output()
            .map_err(|e| e.to_string())?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("nvidia-smi failed: {}", stderr.trim()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut gpus = Vec::new();
        
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(", ").collect();
            if parts.len() < 9 { 
                continue;
            }
            
            let name = parts[0].to_string();
            let utilization = parts[1].parse::<u32>().unwrap_or(0);
            let memory_used = parts[2].parse::<u64>().unwrap_or(0) * 1024 * 1024;
            let memory_total = parts[3].parse::<u64>().unwrap_or(0) * 1024 * 1024;
            let temperature = parts[4].parse::<u32>().unwrap_or(0);
            let power_usage = (parts[5].parse::<f32>().unwrap_or(0.0) * 1000.0) as u32;
            let graphics_clock = parts[6].parse::<u32>().unwrap_or(0);
            let memory_clock = parts[7].parse::<u32>().unwrap_or(0);
            let fan_speed = parts[8].parse::<u32>().ok();
            let driver_version = parts.get(9).unwrap_or(&"Unknown").to_string();
            
            gpus.push(GpuInfo {
                name,
                brand: "NVIDIA".to_string(),
                utilization,
                memory_used,
                memory_total,
                temperature,
                memory_temperature: None,
                power_usage,
                graphics_clock,
                memory_clock,
                fan_speed,
                pci_link_gen: None,
                pci_link_width: None,
                driver_version,
                utilization_history: Vec::new(),
                memory_history: Vec::new(),
            });
        }
        
        Ok(gpus)
    }

    fn get_drm_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        let mut gpus = Vec::new();
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Err("/sys/class/drm not found".to_string());
        }

        for entry in fs::read_dir(drm_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with("card") && !name.contains("-") && name.chars().skip(4).all(|c| c.is_numeric()) {
                let device_path = path.join("device");
                
                if let Ok(vendor_str) = fs::read_to_string(device_path.join("vendor")) {
                    let vendor_id = vendor_str.trim();
                    if vendor_id == "0x1002" {
                        if let Ok(gpu) = self.parse_amd_gpu(&device_path, &name) {
                            gpus.push(gpu);
                        }
                    } else if vendor_id == "0x8086" {
                        if let Ok(gpu) = self.parse_intel_gpu(&path, &device_path, &name) {
                            gpus.push(gpu);
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
    }

    fn parse_amd_gpu(&self, device_path: &Path, card_name: &str) -> Result<GpuInfo, String> {
        let name = fs::read_to_string(device_path.join("product_name"))
             .or_else(|_| fs::read_to_string(device_path.join("product_number")))
             .or_else(|_| fs::read_to_string(device_path.join("device")))
             .unwrap_or_else(|_| format!("AMD GPU ({})", card_name))
             .trim()
             .to_string();

        // Try multiple paths for utilization
        let utilization = fs::read_to_string(device_path.join("gpu_busy_percent"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .or_else(|| {
                fs::read_to_string(device_path.join("busy_percent")) // older kernels
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
            })
             .or_else(|| {
                 fs::read_to_string(device_path.join("device/gpu_busy_percent"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
            })
             .or_else(|| {
                 fs::read_to_string(device_path.join("device/load")) // some drivers use 0-100 load
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
            })
            .unwrap_or(0);

        let (memory_used, memory_total) = self.read_amd_memory(device_path);
        let temperature = self.find_hwmon_temp(device_path).unwrap_or(0);
        let power_usage = self.find_hwmon_power(device_path).unwrap_or(0);

        let mut graphics_clock = self.read_amd_clock(device_path, "pp_dpm_sclk").unwrap_or(0);
        if graphics_clock == 0 {
             // Fallback to hwmon freq inputs
             graphics_clock = self.find_hwmon_clock(device_path, "freq1_input").unwrap_or(0);
        }

        let mut memory_clock = self.read_amd_clock(device_path, "pp_dpm_mclk").unwrap_or(0);
        if memory_clock == 0 {
             memory_clock = self.find_hwmon_clock(device_path, "freq2_input").unwrap_or(0);
        }
        
        if graphics_clock == 0 {
             graphics_clock = self.find_hwmon_clock(device_path, "freq0_input").unwrap_or(0);
        }

        Ok(GpuInfo {
            name,
            brand: "AMD".to_string(),
            utilization,
            memory_used,
            memory_total,
            temperature,
            memory_temperature: None,
            power_usage,
            graphics_clock,
            memory_clock,
            fan_speed: None, 
            pci_link_gen: None,
            pci_link_width: None,
            driver_version: "amdgpu".to_string(),
            utilization_history: Vec::new(),
            memory_history: Vec::new(),
        })
    }
    
    fn read_amd_memory(&self, device_path: &Path) -> (u64, u64) {
        let total = fs::read_to_string(device_path.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
            
        let used = fs::read_to_string(device_path.join("mem_info_vram_used"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
            
        (used, total)
    }

    fn read_amd_clock(&self, device_path: &Path, file_name: &str) -> Option<u32> {
        if let Ok(content) = fs::read_to_string(device_path.join(file_name)) {
            for line in content.lines() {
                if line.contains('*') {
                    for part in line.split_whitespace() {
                        if part.ends_with("Mhz") {
                             let num_str = &part[..part.len()-3];
                             return num_str.parse::<u32>().ok();
                        }
                    }
                }
            }
        }
        None
    }

    fn find_hwmon_clock(&self, device_path: &Path, filename: &str) -> Option<u32> {
        let hwmon_dir = device_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                 let hwmon_path = entry.path();
                 
                 let path = hwmon_path.join(filename);
                 if path.exists() {
                     if let Ok(s) = fs::read_to_string(&path) {
                         if let Ok(val) = s.trim().parse::<u32>() {
                             return Some(val / 1_000_000);
                         }
                     }
                 }
                 
                 if filename.starts_with("freq") {
                     for i in 1..=3 {
                         let alt_path = hwmon_path.join(format!("freq{}_input", i));
                         if alt_path.exists() {
                             if let Ok(s) = fs::read_to_string(&alt_path) {
                                 if let Ok(val) = s.trim().parse::<u32>() {
                                      let mhz = val / 1_000_000;
                                      if mhz > 100 {
                                          return Some(mhz);
                                      }
                                 }
                             }
                         }
                     }
                 }
            }
        }
        None
    }

    fn parse_intel_gpu(&self, card_path: &Path, device_path: &Path, card_name: &str) -> Result<GpuInfo, String> {
        let name = fs::read_to_string(device_path.join("device"))
             .map(|id| format!("Intel Graphics ({})", id.trim()))
             .unwrap_or_else(|_| format!("Intel GPU ({})", card_name));

        let freq_paths = [
            card_path.join("gt/gt0/rps_act_freq_mhz"),
            card_path.join("gt/gt0/rps_cur_freq_mhz"),
            card_path.join("gt/gt0/gt_act_freq_mhz"),
            card_path.join("gt/gt0/gt_cur_freq_mhz"),
            card_path.join("gt_act_freq_mhz"),
            card_path.join("gt_cur_freq_mhz"),
            device_path.join("gt_act_freq_mhz"),
            device_path.join("gt_cur_freq_mhz"),
        ];

        let mut graphics_clock = 0;
        for path in &freq_paths {
            if let Ok(s) = fs::read_to_string(path) {
                if let Ok(val) = s.trim().parse::<u32>() {
                     if graphics_clock == 0 {
                         graphics_clock = val;
                     }
                    if val > 0 {
                        graphics_clock = val;
                        break;
                    }
                }
            }
        }
             
        let temperature = self.find_hwmon_temp(device_path).unwrap_or(0);
        let power_usage = self.find_hwmon_power(device_path).unwrap_or(0);
        
        Ok(GpuInfo {
            name,
            brand: "Intel".to_string(),
            utilization: 0, 
            memory_used: 0,
            memory_total: 0,
            temperature,
            memory_temperature: None,
            power_usage,
            graphics_clock,
            memory_clock: 0,
            fan_speed: None,
            pci_link_gen: None,
            pci_link_width: None,
            driver_version: "i915".to_string(),
            utilization_history: Vec::new(),
            memory_history: Vec::new(),
        })
    }

    fn find_hwmon_temp(&self, device_path: &Path) -> Option<u32> {
        let hwmon_dir = device_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                 let path = entry.path();
                 if path.is_dir() {
                     if let Ok(hwmon_entries) = fs::read_dir(&path) {
                         for hwmon_entry in hwmon_entries.flatten() {
                            let file_name = hwmon_entry.file_name().to_string_lossy().to_string();
                            if file_name.starts_with("temp") && file_name.ends_with("_input") {
                                if let Ok(s) = fs::read_to_string(hwmon_entry.path()) {
                                    if let Ok(val) = s.trim().parse::<u32>() {
                                        return Some(val / 1000);
                                    }
                                }
                            }
                         }
                     }
                 }
            }
        }
        None
    }
    
    fn find_hwmon_power(&self, device_path: &Path) -> Option<u32> {
         let hwmon_dir = device_path.join("hwmon");
        if let Ok(entries) = fs::read_dir(hwmon_dir) {
            for entry in entries.flatten() {
                 let path = entry.path();
                 if path.is_dir() {
                     if let Ok(hwmon_entries) = fs::read_dir(&path) {
                        for hwmon_entry in hwmon_entries.flatten() {
                            let file_name = hwmon_entry.file_name().to_string_lossy().to_string();
                            if file_name.starts_with("power") && (file_name.ends_with("_average") || file_name.ends_with("_input")) {
                                if let Ok(s) = fs::read_to_string(hwmon_entry.path()) {
                                    if let Ok(val) = s.trim().parse::<u32>() {
                                        return Some(val / 1000);
                                    }
                                }
                            }
                        }
                     }
                 }
            }
        }
        None
    }
    
    pub fn get_primary_gpu_utilization(&self, gpus: &[GpuInfo]) -> Option<u32> {
        if gpus.is_empty() {
            None
        } else {
            Some(gpus.iter().map(|g| g.utilization).max().unwrap_or(0))
        }
    }
    
    pub fn update_gpu_history(&mut self, gpus: &[GpuInfo], max_history: usize) {
        let utilizations: Vec<u32> = gpus.iter().map(|g| g.utilization).collect();
        let memory_usage: Vec<u32> = gpus.iter().map(|g| {
            if g.memory_total > 0 {
                ((g.memory_used as f64 / g.memory_total as f64) * 100.0) as u32
            } else {
                0
            }
        }).collect();
        
        self.gpu_history.push_back(utilizations);
        self.gpu_memory_history.push_back(memory_usage);
        
        while self.gpu_history.len() > max_history {
            self.gpu_history.pop_front();
        }
        while self.gpu_memory_history.len() > max_history {
            self.gpu_memory_history.pop_front();
        }
    }
    
    pub fn get_gpu_history_flat(&self) -> Vec<u64> {
        self.gpu_history
            .iter()
            .map(|frame| frame.iter().cloned().max().unwrap_or(0) as u64)
            .collect()
    }
    
    pub fn is_available(&self) -> bool {
        true
    }
}