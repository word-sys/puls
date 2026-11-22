use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::types::SensorInfo;

pub fn is_net_interface_up(iface: &str) -> bool {
    let base = Path::new("/sys/class/net").join(iface);
    if let Ok(state) = fs::read_to_string(base.join("operstate")) {
        let s = state.trim();
        if s == "up" {
            return true;
        }
        if s == "down" {
            return false;
        }
    }
    if let Ok(carrier) = fs::read_to_string(base.join("carrier")) {
        return carrier.trim() == "1";
    }
    iface == "lo"
}

pub struct HwmonScanner {
    max_seen: HashMap<String, f32>,
}

impl HwmonScanner {
    pub fn new() -> Self {
        Self {
            max_seen: HashMap::new(),
        }
    }

    pub fn scan(&mut self) -> Vec<SensorInfo> {
        let mut sensors = Vec::new();
        self.scan_hwmon_dir(Path::new("/sys/class/hwmon"), &mut sensors);

        if !sensors.iter().any(|s| s.sensor_type == "temp") {
            self.scan_thermal_zones(Path::new("/sys/class/thermal"), &mut sensors);
        }

        let type_order = |t: &str| -> u8 {
            match t {
                "temp" => 0,
                "fan" => 1,
                "in" => 2,
                "power" => 3,
                "curr" => 4,
                _ => 5,
            }
        };
        sensors.sort_by(|a, b| {
            type_order(&a.sensor_type)
                .cmp(&type_order(&b.sensor_type))
                .then_with(|| a.label.cmp(&b.label))
        });

        sensors
    }

    fn record_max(&mut self, key: &str, temp: f32) -> f32 {
        let entry = self.max_seen.entry(key.to_string()).or_insert(temp);
        if temp > *entry {
            *entry = temp;
        }
        *entry
    }

    pub fn scan_hwmon_dir(&mut self, hwmon_root: &Path, sensors: &mut Vec<SensorInfo>) {
        let Ok(entries) = fs::read_dir(hwmon_root) else {
            return;
        };

        for entry in entries.flatten() {
            let hwmon_path = entry.path();
            let chip_name = fs::read_to_string(hwmon_path.join("name"))
                .unwrap_or_default()
                .trim()
                .to_string();

            let Ok(files) = fs::read_dir(&hwmon_path) else {
                continue;
            };
            let mut file_names: Vec<String> = files
                .flatten()
                .map(|f| f.file_name().to_string_lossy().to_string())
                .collect();
            file_names.sort();

            for fname in &file_names {
                if fname.starts_with("temp") && fname.ends_with("_input") {
                    let idx = fname.trim_start_matches("temp").trim_end_matches("_input");
                    let Some(current) = fs::read_to_string(hwmon_path.join(fname))
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok())
                        .map(|v| v / 1000.0)
                    else {
                        continue;
                    };

                    let raw_label = fs::read_to_string(hwmon_path.join(format!("temp{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} temp{}", chip_name, idx));

                    let label = if !chip_name.is_empty()
                        && !raw_label.to_lowercase().contains(&chip_name.to_lowercase())
                        && (chip_name == "nvme" || chip_name == "k10temp" || chip_name == "coretemp" || chip_name == "zenpower")
                    {
                        format!("{} {}", chip_name, raw_label)
                    } else {
                        raw_label
                    };

                    if sensors
                        .iter()
                        .any(|s| s.sensor_type == "temp" && s.label == label)
                    {
                        continue;
                    }

                    let hw_limit = fs::read_to_string(hwmon_path.join(format!("temp{}_max", idx)))
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok())
                        .map(|v| v / 1000.0);

                    let hw_crit = fs::read_to_string(hwmon_path.join(format!("temp{}_crit", idx)))
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok())
                        .map(|v| v / 1000.0);

                    let max_seen = self.record_max(&label, current);

                    sensors.push(SensorInfo {
                        label,
                        sensor_type: "temp".to_string(),
                        value: current as f64,
                        unit: "°C".to_string(),
                        temp: current,
                        max: Some(max_seen),
                        limit: hw_limit,
                        critical: hw_crit,
                    });
                    continue;
                }

                if fname.starts_with("fan") && fname.ends_with("_input") {
                    let idx = fname.trim_start_matches("fan").trim_end_matches("_input");
                    let label = fs::read_to_string(hwmon_path.join(format!("fan{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} Fan {}", chip_name, idx));
                    if let Ok(val) = fs::read_to_string(hwmon_path.join(fname)) {
                        if let Ok(rpm) = val.trim().parse::<f64>() {
                            sensors.push(SensorInfo {
                                label,
                                sensor_type: "fan".to_string(),
                                value: rpm,
                                unit: "RPM".to_string(),
                                temp: rpm as f32,
                                max: None,
                                limit: None,
                                critical: None,
                            });
                        }
                    }
                    continue;
                }

                if fname.starts_with("in")
                    && fname.ends_with("_input")
                    && fname[2..].starts_with(|c: char| c.is_ascii_digit())
                {
                    let idx = fname.trim_start_matches("in").trim_end_matches("_input");
                    let label = fs::read_to_string(hwmon_path.join(format!("in{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} Voltage {}", chip_name, idx));
                    if let Ok(val) = fs::read_to_string(hwmon_path.join(fname)) {
                        if let Ok(mv) = val.trim().parse::<f64>() {
                            let volts = mv / 1000.0;
                            sensors.push(SensorInfo {
                                label,
                                sensor_type: "in".to_string(),
                                value: volts,
                                unit: "V".to_string(),
                                temp: volts as f32,
                                max: None,
                                limit: None,
                                critical: None,
                            });
                        }
                    }
                    continue;
                }

                if fname.starts_with("power")
                    && (fname.ends_with("_input") || fname.ends_with("_average"))
                {
                    let suffix = if fname.ends_with("_input") {
                        "_input"
                    } else {
                        "_average"
                    };
                    let idx = fname.trim_start_matches("power").trim_end_matches(suffix);
                    let label = fs::read_to_string(hwmon_path.join(format!("power{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} Power {}", chip_name, idx));
                    if sensors
                        .iter()
                        .any(|s| s.label == label && s.sensor_type == "power")
                    {
                        continue;
                    }
                    if let Ok(val) = fs::read_to_string(hwmon_path.join(fname)) {
                        if let Ok(uw) = val.trim().parse::<f64>() {
                            let watts = uw / 1_000_000.0;
                            sensors.push(SensorInfo {
                                label,
                                sensor_type: "power".to_string(),
                                value: watts,
                                unit: "W".to_string(),
                                temp: watts as f32,
                                max: None,
                                limit: None,
                                critical: None,
                            });
                        }
                    }
                    continue;
                }

                if fname.starts_with("curr") && fname.ends_with("_input") {
                    let idx = fname.trim_start_matches("curr").trim_end_matches("_input");
                    let label = fs::read_to_string(hwmon_path.join(format!("curr{}_label", idx)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} Current {}", chip_name, idx));
                    if let Ok(val) = fs::read_to_string(hwmon_path.join(fname)) {
                        if let Ok(ma) = val.trim().parse::<f64>() {
                            let amps = ma / 1000.0;
                            sensors.push(SensorInfo {
                                label,
                                sensor_type: "curr".to_string(),
                                value: amps,
                                unit: "A".to_string(),
                                temp: amps as f32,
                                max: None,
                                limit: None,
                                critical: None,
                            });
                        }
                    }
                }
            }
        }
    }

    fn scan_thermal_zones(&mut self, thermal_root: &Path, sensors: &mut Vec<SensorInfo>) {
        let Ok(entries) = fs::read_dir(thermal_root) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("thermal_zone") {
                continue;
            }
            let path = entry.path();
            let Some(temp) = fs::read_to_string(path.join("temp"))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|v| v / 1000.0)
            else {
                continue;
            };
            let label = fs::read_to_string(path.join("type"))
                .map(|s| s.trim().to_string())
                .unwrap_or(name);
            let max_seen = self.record_max(&label, temp);
            sensors.push(SensorInfo {
                label,
                sensor_type: "temp".to_string(),
                value: temp as f64,
                unit: "°C".to_string(),
                temp,
                max: Some(max_seen),
                limit: None,
                critical: None,
            });
        }
    }
}

pub fn extract_cpu_core_temps(sensors: &[SensorInfo]) -> (Vec<(u32, f32)>, Option<f32>) {
    let mut core_sensors: Vec<(u32, f32)> = sensors
        .iter()
        .filter(|s| s.sensor_type == "temp")
        .filter_map(|s| {
            let lower = s.label.to_lowercase();
            if lower.contains("core") && !lower.contains("package") {
                let id = lower
                    .split_whitespace()
                    .filter_map(|tok| tok.parse::<u32>().ok())
                    .next()?;
                Some((id, s.temp))
            } else {
                None
            }
        })
        .collect();

    core_sensors.sort_by_key(|(id, _)| *id);

    let global_temp = sensors
        .iter()
        .filter(|s| s.sensor_type == "temp")
        .find(|s| {
            let lower = s.label.to_lowercase();
            lower.contains("tctl")
                || lower.contains("package")
                || lower.contains("tdie")
                || lower.contains("k10temp")
                || lower.contains("zenpower")
        })
        .map(|s| s.temp)
        .or_else(|| core_sensors.first().map(|(_, t)| *t))
        .or_else(|| {
            sensors
                .iter()
                .find(|s| {
                    s.sensor_type == "temp"
                        && (s.label.to_lowercase().contains("cpu")
                            || s.label.to_lowercase().contains("x86_pkg"))
                })
                .map(|s| s.temp)
        });

    (core_sensors, global_temp)
}

pub fn find_memory_temp(sensors: &[SensorInfo]) -> Option<f32> {
    sensors
        .iter()
        .find(|s| {
            s.sensor_type == "temp" && {
                let l = s.label.to_lowercase();
                l.contains("mem") || l.contains("dimm") || l.contains("dram")
            }
        })
        .map(|s| s.temp)
}

pub fn find_disk_temp(sensors: &[SensorInfo], block_dev: &str) -> Option<f32> {
    let dev_lower = block_dev.to_lowercase();
    sensors
        .iter()
        .find(|s| {
            s.sensor_type == "temp" && {
                let l = s.label.to_lowercase();
                l.contains(&dev_lower) || dev_lower.contains(&l)
            }
        })
        .map(|s| s.temp)
        .or_else(|| {
            if dev_lower.starts_with("nvme") {
                sensors
                    .iter()
                    .find(|s| s.sensor_type == "temp" && s.label.to_lowercase().contains("nvme"))
                    .map(|s| s.temp)
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hwmon_synthetic_scan_and_max_tracking() {
        let dir = std::env::temp_dir().join(format!("puls_hwmon_test_{}", std::process::id()));
        let hw0 = dir.join("hwmon0");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&hw0).unwrap();

        fs::write(hw0.join("name"), "coretemp\n").unwrap();
        fs::write(hw0.join("temp1_label"), "Package id 0\n").unwrap();
        fs::write(hw0.join("temp1_input"), "55000\n").unwrap();
        fs::write(hw0.join("temp1_max"), "85000\n").unwrap();
        fs::write(hw0.join("temp1_crit"), "100000\n").unwrap();
        fs::write(hw0.join("temp2_label"), "Core 0\n").unwrap();
        fs::write(hw0.join("temp2_input"), "52000\n").unwrap();
        fs::write(hw0.join("fan1_input"), "1450\n").unwrap();

        let mut scanner = HwmonScanner::new();
        let mut sensors = Vec::new();
        scanner.scan_hwmon_dir(&dir, &mut sensors);
        assert_eq!(sensors.len(), 3);

        let (cores, global) = extract_cpu_core_temps(&sensors);
        assert_eq!(cores, vec![(0, 52.0)]);
        assert_eq!(global, Some(55.0));

        fs::write(hw0.join("temp1_input"), "62000\n").unwrap();
        let mut sensors2 = Vec::new();
        scanner.scan_hwmon_dir(&dir, &mut sensors2);
        let pkg = sensors2.iter().find(|s| s.label.contains("Package")).unwrap();
        assert_eq!(pkg.temp, 62.0);
        assert_eq!(pkg.max, Some(62.0));

        fs::write(hw0.join("temp1_input"), "48000\n").unwrap();
        let mut sensors3 = Vec::new();
        scanner.scan_hwmon_dir(&dir, &mut sensors3);
        let pkg3 = sensors3.iter().find(|s| s.label.contains("Package")).unwrap();
        assert_eq!(pkg3.temp, 48.0);
        assert_eq!(pkg3.max, Some(62.0));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_net_interface_up_loopback() {
        assert!(is_net_interface_up("lo"));
    }
}
