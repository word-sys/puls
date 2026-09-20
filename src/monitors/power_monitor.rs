use crate::types::BatteryInfo;
use std::fs;
use std::path::Path;

pub struct PowerMonitor {
    power_supply_path: String,
}

impl PowerMonitor {
    pub fn new() -> Self {
        Self {
            power_supply_path: "/sys/class/power_supply".to_string(),
        }
    }

    pub fn get_battery_info(&self) -> Option<BatteryInfo> {
        let base_path = Path::new(&self.power_supply_path);
        if !base_path.exists() {
            return None;
        }

        let entries = match fs::read_dir(base_path) {
            Ok(e) => e,
            Err(_) => return None,
        };

        let mut ac_online = false;
        let mut battery_paths = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            let supply_type = fs::read_to_string(path.join("type"))
                .unwrap_or_default()
                .trim()
                .to_string();

            if supply_type == "Mains" || name.starts_with("AC") || name.starts_with("ADP") {
                if let Ok(online_str) = fs::read_to_string(path.join("online")) {
                    if online_str.trim() == "1" {
                        ac_online = true;
                    }
                }
            } else if supply_type == "Battery" || name.starts_with("BAT") {
                battery_paths.push((name, path));
            }
        }

        // Use primary battery (e.g. BAT0 or BAT1)
        if let Some((name, bat_path)) = battery_paths.into_iter().next() {
            let status = fs::read_to_string(bat_path.join("status"))
                .unwrap_or_else(|_| "Unknown".to_string())
                .trim()
                .to_string();

            let capacity = fs::read_to_string(bat_path.join("capacity"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);

            let capacity_level = fs::read_to_string(bat_path.join("capacity_level"))
                .ok()
                .map(|s| s.trim().to_string());

            let technology = fs::read_to_string(bat_path.join("technology"))
                .ok()
                .map(|s| s.trim().to_string());

            let model_name = fs::read_to_string(bat_path.join("model_name"))
                .ok()
                .map(|s| s.trim().to_string());

            let manufacturer = fs::read_to_string(bat_path.join("manufacturer"))
                .ok()
                .map(|s| s.trim().to_string());

            let cycle_count = fs::read_to_string(bat_path.join("cycle_count"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .filter(|&c| c > 0);

            let voltage_now_uv = fs::read_to_string(bat_path.join("voltage_now"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());

            let voltage_volts = voltage_now_uv.map(|v| v as f64 / 1_000_000.0);

            // Check power_now (uW) or current_now (uA) * voltage_now (uV)
            let power_watts = if let Ok(s) = fs::read_to_string(bat_path.join("power_now")) {
                s.trim().parse::<f64>().ok().map(|p| p / 1_000_000.0)
            } else if let (Some(v), Ok(c_str)) = (voltage_now_uv, fs::read_to_string(bat_path.join("current_now"))) {
                c_str.trim().parse::<u64>().ok().map(|c| (c as f64 * v as f64) / 1_000_000_000_000.0)
            } else {
                None
            };

            // Energy calculations (Wh)
            let (energy_now_wh, energy_full_wh, energy_design_wh) = if let Ok(e_now) = fs::read_to_string(bat_path.join("energy_now")) {
                let now = e_now.trim().parse::<f64>().ok().map(|e| e / 1_000_000.0);
                let full = fs::read_to_string(bat_path.join("energy_full"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|e| e / 1_000_000.0);
                let design = fs::read_to_string(bat_path.join("energy_full_design"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|e| e / 1_000_000.0);
                (now, full, design)
            } else if let (Some(v), Ok(c_now)) = (voltage_now_uv, fs::read_to_string(bat_path.join("charge_now"))) {
                let now = c_now.trim().parse::<f64>().ok().map(|c| (c * v as f64) / 1_000_000_000_000.0);
                let full = fs::read_to_string(bat_path.join("charge_full"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|c| (c * v as f64) / 1_000_000_000_000.0);
                let design = fs::read_to_string(bat_path.join("charge_full_design"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|c| (c * v as f64) / 1_000_000_000_000.0);
                (now, full, design)
            } else {
                (None, None, None)
            };

            let health_percent = match (energy_full_wh, energy_design_wh) {
                (Some(full), Some(design)) if design > 0.0 => {
                    Some(((full / design) * 100.0) as f32)
                }
                _ => None,
            };

            // CPU Scaling governor
            let cpu_governor = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
                .ok()
                .map(|s| s.trim().to_string());
            let cpu_driver = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver")
                .ok()
                .map(|s| s.trim().to_string());

            Some(BatteryInfo {
                name,
                status,
                capacity,
                capacity_level,
                power_watts,
                voltage_volts,
                energy_now_wh,
                energy_full_wh,
                energy_design_wh,
                cycle_count,
                health_percent,
                technology,
                model_name,
                manufacturer,
                ac_online,
                cpu_governor,
                cpu_driver,
            })
        } else {
            None
        }
    }
}

impl Default for PowerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_monitor_creation() {
        let monitor = PowerMonitor::new();
        let info = monitor.get_battery_info();
        if let Some(bat) = info {
            assert!(!bat.name.is_empty());
            assert!(bat.capacity <= 100);
            if let Some(hp) = bat.health_percent {
                assert!(hp > 0.0 && hp <= 150.0);
            }
        }
    }
}
