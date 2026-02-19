# Changelog

All notable changes to this project will be documented in this file.

## [v0.7.1] - 2026-02-19

### Fixed
- **Sensors Tab**: Updated sensors tab to show all sensors
- **CPU Temp N/A Issue**: CPU Temp monitoring now also looks into /sys/class/hwmon/*/name for k10temp, coretemp, k8temp, zenpower
- **Docker Fix**: Untested but i think i fixed legacy client errors for newer Docker APIs, otherwise will be fixed in 0.7.2

## [v0.7.0] - 2026-02-18

### Added
- **Real-Time Sensors**: Temperatures and sensor data refresh in real-time and used on different tabs
- **Disk SMART Data**: NVMe health percentage, power cycle count, and disk type (NVMe/SSD/HDD) shown on Disks tab
- **Sensors Tab Redesign**: Sensors tried to be grouped by category (CPU, GPU, Memory, Disk, Fan, Other) with visual temperature bars
- **CPU Cores**: Enlarged gauges with borders and temperature display in info panel
- **Memory Temperature**: Temperature row added to Memory tab details, not every computer gaves this info so dont expect much
- **Log Detail Modal**: Styled to match service status modal (larger, consistent borders)

### Fixed
- **Docker Compatibility**: Updated bollard to v0.19, fixed legacy client errors for newer Docker APIs
- **AMD GPU Monitoring**: (I hope again) Fixed 0Hz/0Usage on AMD cards with hwmon sensor fallback
- **NVMe Temperature**: Added sysfs hwmon fallback when component label matching fails

## [v0.6.2] - 2026-02-17

### Added
- **Container Logs**: Added a new feature to view Docker container logs directly in the UI (press 'l' in the containers tab).
- **Quick Launch**: Release executable PULS file now opens terminal for itself, result is working like AppImage application but executable way.

### Fixes
- **Docker Compatibility**: Updated bollard to v0.19 and refactored codebase to fix legacy client errors and support newer Docker APIs.
- **AMD GPU Monitoring**: (I hope) Fixed 0Hz/0Usage reporting on AMD cards by implementing a fallback to hwmon sensors when legacy pp_dpm_* files are missing.
- **Build Cleanliness**: Removed unused "add implementation later" code and parameters, resolving compiler warnings and lowering binary size.

## [v0.6.1] - 2026-02-08

### Added
- **GPU Memory Monitoring**: Added memory usage tracking and history chart for NVIDIA, AMD, and Intel GPUs.
- **AMD GPU Fix**: Robust fallback parsing for AMD GPU utilization to resolve "zeros" reporting issue.
- **Debian Packaging**: Support for building `.deb` packages using pre-compiled musl binaries.
- **Detailed Resource Info**: Added CPU efficiency, Swap usage, and more detailed system status line.

### Changed
- **UI Layout**: Expanded Process list and reduced Container list for better focus on processes.
- **Summary Bar**: Increased height to 4 lines and restored borders to Network/Disk I/O sections with sparklines for better visibility.
- **Performance**: Optimized system monitoring.
- **UI Performance**: Adjusted refresh rate to 30 FPS (33ms) for fixing rendering and data problems.

### Fixed
- **Docker**: Resolved "Legacy error" by updating `bollard` dependency to 0.18.
- **Service Management**: Fixed issue where stopped services would disappear from the list. Services are now enumerated using `list-unit-files` to ensure all installed services are visible regardless of state.

## [v0.6.0] - 2026-01-28

### Added
- **Multi-Vendor GPU Support**: Support for NVIDIA (via `nvidia-smi`), AMD, and Intel GPUs (via `/sys/class/drm`).
- **Multi-GPU Monitoring**: Support for tracking and displaying telemetry for multiple GPUs simultaneously.
- **GPU History Visualization**: Real-time utilization history rendered using Braille dot patterns on the dashboard.
- **"General" Process Sorting**: New sorting mode that combines CPU and Memory usage for a balanced resource view (Ctrl+G).
- **Service Action Confirmations**: Confirmation dialogs for stopping system services to prevent accidental interruptions.
- **Sudo Privilege Detection**: Automatic detection of root privileges with read-only fallback for non-root users.

### Fixed
- **Process Kill Logic**: Fixed the `kill` command to correctly target the currently highlighted process in the list.
- **Dashboard UI**: Restored missing navigation hints in the footer.
- **UI Consistency**: Standardized footer keybindings across all tabs.

### Changed
- Refactored GPU monitoring to be more resilient and support multi-vram/multi-core telemetry.
- Updated Dashboard layout for better information density.
- Improved system service management safety checks.
