# Changelog

All notable changes to this project will be documented in this file

## [v0.7.1] - 2026-02-23

### Fixed
- **Sensors Tab**: Updated sensors tab to show all sensors correctly
- **CPU Temp N/A Issue**: CPU Temp monitoring now also looks into /sys/class/hwmon/*/name for k10temp, coretemp, k8temp, zenpower
- **Docker Change**: For some reason, to access Docker containers info for monitoring requires PULS ran with "sudo". Until i find a solution to that problem users has Docker containers and wants to monitor them needed to run PULS with "sudo"
- **Color Fix**: Some places used secondary color instead of primary color, all fixed now

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
- **Container Logs**: Added a new feature to view Docker container logs directly in the UI (press 'l' in the containers tab)
- **Quick Launch**: Release executable PULS file now opens terminal for itself, result is working like AppImage application but executable way

### Fixes
- **Docker Compatibility**: Updated bollard to v0.19 and refactored codebase to fix legacy client errors and support newer Docker APIs
- **AMD GPU Monitoring**: (I hope) Fixed 0Hz/0Usage reporting on AMD cards by implementing a fallback to hwmon sensors when legacy pp_dpm_* files are missing
- **Build Cleanliness**: Removed unused "add implementation later" code and parameters, resolving compiler warnings and lowering binary size

## [v0.6.1] - 2026-02-08

### Added
- **GPU Memory Monitoring**: Added memory usage tracking and history chart for NVIDIA, AMD, and Intel GPUs
- **AMD GPU Fix**: Robust fallback parsing for AMD GPU utilization to resolve "zeros" reporting issue
- **Debian Packaging**: Support for building `.deb` packages using pre-compiled musl binaries
- **Detailed Resource Info**: Added CPU efficiency, Swap usage, and more detailed system status line

### Changed
- **UI Layout**: Expanded Process list and reduced Container list for better focus on processes
- **Summary Bar**: Increased height to 4 lines and restored borders to Network/Disk I/O sections with sparklines for better visibility
- **Performance**: Optimized system monitoring
- **UI Performance**: Adjusted refresh rate to 30 FPS (33ms) for fixing rendering and data problems

### Fixed
- **Docker**: Resolved "Legacy error" by updating `bollard` dependency to 0.18
- **Service Management**: Fixed issue where stopped services would disappear from the list. Services are now enumerated using `list-unit-files` to ensure all installed services are visible regardless of state

## [v0.6.0] - 2026-01-28

### Added
- **Multi-Vendor GPU Support**: Support for NVIDIA (via `nvidia-smi`), AMD, and Intel GPUs (via `/sys/class/drm`)
- **Multi-GPU Monitoring**: Support for tracking and displaying telemetry for multiple GPUs simultaneously
- **GPU History Visualization**: Real-time utilization history rendered using Braille dot patterns on the dashboard
- **"General" Process Sorting**: New sorting mode that combines CPU and Memory usage for a balanced resource view (Ctrl+G)
- **Service Action Confirmations**: Confirmation dialogs for stopping system services to prevent accidental interruptions
- **Sudo Privilege Detection**: Automatic detection of root privileges with read-only fallback for non-root users

### Fixed
- **Process Kill Logic**: Fixed the `kill` command to correctly target the currently highlighted process in the list
- **Dashboard UI**: Restored missing navigation hints in the footer
- **UI Consistency**: Standardized footer keybindings across all tabs

### Changed
- Refactored GPU monitoring to be more resilient and support multi-vram/multi-core telemetry
- Updated Dashboard layout for better information density
- Improved system service management safety checks

## [v0.5.1] - 2026-01-21

### Added
- **Backwards Compatibility**: Added backwards compatibility, after this update from Debian 10 or Ubuntu 20.04 up to Bleeding Edge Linux distributions can use PULS

## [v0.5.0] - 2026-01-20

### Added
- **Memory Tab**: Added Memory Tab (Key 4) with RAM/Swap usage and breakdown
- **Containers Tab**: Restored Containers Tab (Key =) with graceful handling for inactive Docker services
- **"Detailed CPU Info"**: Added detailed CPU information block (Model, Cores) and usage chart to CPU tab
- **Disk I/O**: Added Disk I/O rates and IOPS metrics to Disks tab

### Fixed
- **Process Logic**: Fixed "0 Running Processes" issue; active processes now tracked correctly
- **UI Layout**: Widened columns for Disk Devices and GPU names to fix text truncation
- **UI Theme**: Updated UI theme for better high-contrast support
- **Shortcuts and Translations**: Standardized tab shortcuts and improved Turkish translations

## [v0.4.0] - 2025-12-30

### Added
- **Turkish**: Added Turkish language support
- **Services**: Added system service editing, watching tool and system logs tool
- **GRUB**: Added GRUB editing tool

### Fixed
- **Stability**: Improved stability
- **MHz Bug**: Corrected CPU Mhz calculation

## [v0.3.0] - 2025-08-19

### Fixed
- **CPU Usage**: Corrected CPU usage calculation per process
- **Smooth UI**: 1-second refresh rate with smooth 60 FPS UI
- **Memory Leaks**: Reduced memory footprint and eliminated memory leaks

## [v0.2.0] - 2025-08-7

### Added
- **Safe Mode**: Added Safe Mode (use --safe)
- **Optimizations**: Added Optimizations

## [v0.1.0] - 2025-08-6

### Added
- **Initial Release**: Initial release of PULS
