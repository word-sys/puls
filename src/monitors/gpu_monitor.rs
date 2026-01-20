use crate::types::GpuInfo;
use std::collections::VecDeque;

pub struct GpuMonitor {
    #[cfg(feature = "nvidia-gpu")]
    nvml: Result<nvml_wrapper::Nvml, String>,
    
    #[cfg(feature = "amd-gpu")]
    amd_initialized: bool,
    
    gpu_history: VecDeque<Vec<u32>>,
    last_update: std::time::Instant,
}

impl GpuMonitor {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "nvidia-gpu")]
            nvml: Self::init_nvidia(),
            
            #[cfg(feature = "amd-gpu")]
            amd_initialized: Self::init_amd(),
            
            gpu_history: VecDeque::new(),
            last_update: std::time::Instant::now(),
        }
    }
    
    #[cfg(feature = "nvidia-gpu")]
    fn init_nvidia() -> Result<nvml_wrapper::Nvml, String> {
        nvml_wrapper::Nvml::init().map_err(|e| format!("NVML initialization failed: {}", e))
    }
    
    #[cfg(not(feature = "nvidia-gpu"))]
    fn init_nvidia() -> Result<(), String> {
        Err("NVIDIA GPU support not compiled".to_string())
    }
    
    #[cfg(feature = "amd-gpu")]
    fn init_amd() -> bool {
        false
    }
    
    #[cfg(not(feature = "amd-gpu"))]
    fn init_amd() -> bool {
        false
    }
    
    pub fn get_gpu_info(&mut self) -> Result<Vec<GpuInfo>, String> {
        let mut gpus = Vec::new();
        
        #[cfg(feature = "nvidia-gpu")]
        if let Ok(ref nvml) = self.nvml {
            match self.get_nvidia_gpus(nvml) {
                Ok(mut nvidia_gpus) => gpus.append(&mut nvidia_gpus),
                Err(e) => return Err(format!("NVIDIA GPU error: {}", e)),
            }
        }
        
        #[cfg(feature = "amd-gpu")]
        if self.amd_initialized {
            match self.get_amd_gpus() {
                Ok(mut amd_gpus) => gpus.append(&mut amd_gpus),
                Err(e) => eprintln!("AMD GPU warning: {}", e),
            }
        }
                
        if gpus.is_empty() {
            #[cfg(feature = "nvidia-gpu")]
            if let Err(ref e) = self.nvml {
                return Err(e.clone());
            }
            
            Err("No supported GPUs found".to_string())
        } else {
            Ok(gpus)
        }
    }
    
    #[cfg(feature = "nvidia-gpu")]
    fn get_nvidia_gpus(&self, nvml: &nvml_wrapper::Nvml) -> Result<Vec<GpuInfo>, String> {
        let device_count = nvml.device_count().map_err(|e| e.to_string())?;
        let mut gpus = Vec::new();
        
        for i in 0..device_count {
            let device = nvml.device_by_index(i).map_err(|e| e.to_string())?;
            
            let name = device.name().map_err(|e| e.to_string())?;
            let memory_info = device.memory_info().map_err(|e| e.to_string())?;
            let utilization = device.utilization_rates()
                .map_err(|e| e.to_string())?;
            
            let temperature = device.temperature(
                nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu
            ).map_err(|e| e.to_string())?;
            
            let power_usage = device.power_usage().map_err(|e| e.to_string())?;
            
            let graphics_clock = device.clock_info(
                nvml_wrapper::enum_wrappers::device::Clock::Graphics
            ).map_err(|e| e.to_string())?;
            
            let memory_clock = device.clock_info(
                nvml_wrapper::enum_wrappers::device::Clock::Memory
            ).map_err(|e| e.to_string())?;
            
            let fan_speed = device.fan_speed(0).ok();

            let driver_version = nvml.sys_driver_version()
                .unwrap_or_else(|_| "Unknown".to_string());
            
            let memory_temperature = None;
                /*
                // Newer NVML/Drivers might support Memory temperature
                // Falling back to GPU temp if not specifically available or use separate sensor query
                let memory_temperature = device.temperature(
                     nvml_wrapper::enum_wrappers::device::TemperatureSensor::Memory
                ).ok();
                */

                let pci_link_gen = device.current_pcie_link_gen().ok();
                let pci_link_width = device.current_pcie_link_width().ok();
            
                gpus.push(GpuInfo {
                    name,
                    brand: "NVIDIA".to_string(),
                    utilization: utilization.gpu,
                    memory_used: memory_info.used,
                    memory_total: memory_info.total,
                    temperature,
                    memory_temperature,
                    power_usage,
                    graphics_clock,
                    memory_clock,
                    fan_speed,
                    pci_link_gen,
                    pci_link_width,
                    driver_version,
                });
        }
        
        Ok(gpus)
    }
    
    #[cfg(not(feature = "nvidia-gpu"))]
    fn get_nvidia_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        Err("NVIDIA support not compiled".to_string())
    }
    
    #[cfg(feature = "amd-gpu")]
    fn get_amd_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        let mut gpus = Vec::new(); 
        use std::fs;
        use std::path::Path;
        
        for card_dir in fs::read_dir("/sys/class/drm/").map_err(|e| e.to_string())? {
            let card_dir = card_dir.map_err(|e| e.to_string())?;
            let card_name = card_dir.file_name();
            let card_name_str = card_name.to_string_lossy();
            
            if card_name_str.starts_with("card") && !card_name_str.contains("-") {
                let device_path = card_dir.path().join("device");
                
                if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                    if vendor.trim() == "0x1002" {
                        let gpu_info = self.parse_amd_gpu_info(&device_path, &card_name_str)?;
                        gpus.push(gpu_info);
                    }
                }
            }
        }
        
        if gpus.is_empty() {
            Err("No AMD GPUs found".to_string())
        } else {
            Ok(gpus)
        }
    }
    
    #[cfg(feature = "amd-gpu")]
    fn parse_amd_gpu_info(&self, device_path: &Path, card_name: &str) -> Result<GpuInfo, String> {
        use std::fs;
        
        let name = fs::read_to_string(device_path.join("product_name"))
            .or_else(|_| fs::read_to_string(device_path.join("device")))
            .unwrap_or_else(|_| format!("AMD GPU ({})", card_name))
            .trim()
            .to_string();
        
        let utilization = fs::read_to_string(device_path.join("gpu_busy_percent"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        
        let (memory_used, memory_total) = self.read_amd_memory_info(device_path);
        
        let temperature = self.read_amd_temperature(device_path).unwrap_or(0);
        
        let power_usage = fs::read_to_string(device_path.join("power_dpm_force_performance_level"))
            .ok()
            .and_then(|_| Some(0))
            .unwrap_or(0);
        
        Ok(GpuInfo {
            name,
            brand: "AMD".to_string(),
            utilization,
            memory_used,
            memory_total,
            temperature,
            power_usage,
            graphics_clock: 0,
            memory_clock: 0,   
            fan_speed: None,   
            pci_link_gen: None,
            pci_link_width: None,
            memory_temperature: None,
            driver_version: "amdgpu".to_string(), 
        })
    }
    
    #[cfg(feature = "amd-gpu")]
    fn read_amd_memory_info(&self, device_path: &Path) -> (u64, u64) {
        use std::fs;
        
        if let Ok(vram_total) = fs::read_to_string(device_path.join("mem_info_vram_total")) {
            if let Ok(total) = vram_total.trim().parse::<u64>() {
                let used = fs::read_to_string(device_path.join("mem_info_vram_used"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .unwrap_or(0);
                return (used, total);
            }
        }
        
        (0, 0)
    }
    
    #[cfg(feature = "amd-gpu")]
    fn read_amd_temperature(&self, device_path: &Path) -> Option<u32> {
        use std::fs;
        
        if let Ok(hwmon_dir) = device_path.join("hwmon").read_dir() {
            for hwmon_entry in hwmon_dir.flatten() {
                let temp_path = hwmon_entry.path().join("temp1_input");
                if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                    if let Ok(temp_millic) = temp_str.trim().parse::<u32>() {
                        return Some(temp_millic / 1000); 
                    }
                }
            }
        }
        
        None
    }
    
    #[cfg(not(feature = "amd-gpu"))]
    fn get_amd_gpus(&self) -> Result<Vec<GpuInfo>, String> {
        Err("AMD GPU support not compiled".to_string())
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
        
        self.gpu_history.push_back(utilizations);
        while self.gpu_history.len() > max_history {
            self.gpu_history.pop_front();
        }
    }
    
    pub fn get_gpu_history_flat(&self) -> Vec<u64> {
        self.gpu_history
            .iter()
            .flat_map(|frame| frame.iter().map(|&util| util as u64))
            .collect()
    }
    
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "nvidia-gpu")]
        if self.nvml.is_ok() {
            return true;
        }
        
        #[cfg(feature = "amd-gpu")]
        if self.amd_initialized {
            return true;
        }
        
        false
    }
}

impl Default for GpuMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_monitor_creation() {
        let monitor = GpuMonitor::new();
        assert!(true);
    }
    
    #[test]
    fn test_gpu_history() {
        let mut monitor = GpuMonitor::new();
        let fake_gpus = vec![
            GpuInfo {
                utilization: 50,
                ..Default::default()
            }
        ];
        
        monitor.update_gpu_history(&fake_gpus, 10);
        assert_eq!(monitor.gpu_history.len(), 1);
        
        let history = monitor.get_gpu_history_flat();
        assert_eq!(history, vec![50u64]);
    }
}