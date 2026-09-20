# Changelog

All notable changes to this project will be documented in this file

## [v0.9.4] - 2026-09-20

### Added
- **Multi-Language Expansion (7 Languages)**: Added complete localization and automatic environment detection for English (default), Turkish, French, German, Spanish, Italian, and Russian. Supported runtime cycling with hotkeys (`L` / `l`) and from the settings dialog
- **Granular Settings Modal & Configuration Persistence**: Added an interactive Settings dialog (`F2` / `Shift+S` / `s`) supporting direct configuration of Interface Language, Color Theme, Default Startup Tab, Temperature Units (Celsius / Fahrenheit), Refresh Rate (500ms - 5000ms), and Process Tree View default. Automatically persisted across sessions to `~/.config/puls/config.ini`
- **Terminal Mouse Event Capture**: Full mouse click and scroll support across the TUI. Left click on tab titles to navigate, click the Settings button in the header, and scroll tables, logs, and modal windows using the mouse wheel. Protected with a safe panic hook to ensure terminal state is cleanly restored
- **Hierarchical Process Tree View**: Added parent-child process tree mode (`t` toggle) with branch tree glyphs (`├─`, `└─`) for intuitive hierarchy navigation
- **POSIX Signal Selector & Dynamic Niceness**: Added interactive process signal selector modal (`k` / `F9`) supporting SIGTERM, SIGKILL, SIGHUP, SIGSTOP, SIGCONT, and SIGINT. Added dynamic priority renicing with `[` (increase priority / lower nice) and `]` (decrease priority / raise nice)
- **Extended Process Inspector**: Deep-dive process inspection modal (`Enter`) displaying open file descriptor counts (`/proc/[pid]/fd`), active socket counts, and per-thread metrics with thread CPU utilization (`/proc/[pid]/task`)
- **Filesystem Inodes & Per-Process Disk I/O**: Real-time per-process disk read/write bandwidth calculations (`/proc/[pid]/io`). Partition view now reports inode capacity, usage percentages, and mount flags via `statvfs`
- **Network Sockets & Active Connection Mapping**: Parsed native `/proc/net/tcp`, `/proc/net/udp`, `/proc/net/tcp6`, and `/proc/net/udp6` to track active network sockets, remote endpoints, connection states, and mapped owning PIDs
- **Container Management & Logs**: Added interactive container management in the Containers tab (`s` start, `x` stop, `r` restart, `p` pause) and live container log streaming (`Enter` / `l`). Supported Podman socket detection fallback
- **GPU Extended Telemetry & Throttling Flags**: Added power limit and current power draw, PCIe link generation and width indicators (current vs maximum), fan RPM, and hardware thermal throttling indicators for NVIDIA, AMD, and Intel GPUs
- **Hardware Power & System Administration**: Added battery telemetry (charge percentage, capacity, wattage, health, AC status via `/sys/class/power_supply/`), CPU frequency scaling governors, systemd scheduled timers overview (`systemctl list-timers`), active user login sessions (`who`/`utmp`), and `/var/run/reboot-required` pending reboot detection
- **Multi-Core Scalability & NUMA Node Awareness**: Adaptive grid layouts for systems with up to 128+ logical cores. Parsed NUMA node topologies (`/sys/devices/system/node/`) with per-node core affinity and memory distribution
- **New Color Themes**: Added Dracula, Solarized Dark, and High Contrast themes alongside Default, Dark Blue, and Light (`THEME_COUNT = 6`)
- **Multi-Architecture Release Pipeline**: CI/CD automation for building static MUSL binaries and Debian packages for both `x86_64` (AMD64) and `aarch64` (ARM64) with buildinfo, changes, and SHA256 checksums, builds from now on will be more improved

### Changed
- **No New Dependencies**: All new features implemented natively using standard library interfaces, `/proc`, `/sys`, and core Unix APIs without adding external crates
- **Optimized Tokio Features**: Trimmed Tokio features to only required runtime modules (`rt-multi-thread`, `time`, `sync`, `macros`) reducing binary size
- **Documentation Changes**: Documentation changed to 7 interconnected README files (`README.md`, `README.tr.md`, `README.fr.md`, `README.de.md`, `README.es.md`, `README.it.md`, `README.ru.md`)

## [v0.9.3] - 2026-07-19

### Added
- **Language Support**: Added full Turkish language support

## [v0.9.2] - 2026-06-04

### Added
- **Initialization Telemetry**: Added a `--telemetry` command line option to print startup initialization logs 

### Changed
- **Lazy Loading**: Deferred retrieval of system services, logs, boot history, and GRUB config until their respective tabs are actively opened
- **Fast Journal Queries**: Added the `--boot=0` parameter to journalctl logs command by default to only load log entries from the current boot state, drastically increasing query performance

### Fixed
- **Shortcut Interference**: Protected global TUI keyboard shortcuts (such as `p` for pausing, `t` for theme cycles, etc.) in editing states, allowing typing these characters in the process filter, log filter, and GRUB/System configuration fields

## [v0.9.1] - 2026-05-27

### Added
- **Language Auto-Detection**: PULS now reads `LANG`/`LC_ALL` on launch and automatically selects Turkish or English
- **Interactive Process Filtering**: Press `/` on the Process tab to filter by name in real-time; `Esc` clears the filter
- **Service Log Viewer**: Press `g` on the Services tab to view the last 50 `journald` log lines for the selected service
- **Diagnostics Panel**: Dashboard now highlights system anomalies (high CPU temp, memory pressure, storage critical) inline
- **GPU Dashboard Summary**: GPU utilization and temperature shown directly in the dashboard overview header
- **L1/L2/L3 Cache Info**: CPU tab now shows L1 data/instruction, L2, and L3 cache sizes parsed from `/sys/devices/system/cpu/cpu0/cache/`
- **Transactional GRUB Editor**: Edits are staged in memory; pressing `u` opens a comparison modal showing all pending changes before any write; requires sudo

### Changed
- **Dependency Reduction**: Replaced `users`, `chrono`, `clap`, and `parking_lot` with standard library code and custom Unix FFI helpers
- **Tab Footer Hints**: Footer key indicators now show controls accurate to each active tab
- **Config Column Layout**: Config table columns changed to percentage-based widths for better readability
- **TTY-Safe Symbols**: All Unicode emoji/symbols replaced with ASCII alternatives (`[+]`, `[*]`, `[-]`, `->`, `v`, `^`) for terminal compatibility

### Fixed
- **Docker Tab Navigation**: Up/Down selection and automatic first-row focus now work correctly on the containers tab
- **Number Keys During Edit**: Pressing digit keys while editing a config field no longer switches tabs

## [v0.9.0] - 2026-05-06

### Added
- **CPU Telemetry**: Added hardware metadata parsing for CPU Vendor, Family, L3 Cache, BogoMIPS, and Virtualization status to the CPU tab
- **GPU Tab Refinements**: Restructured the GPU interface to prevent device name truncation and added PCIe link generation/width details
- **Memory Visibility**: Explicitly labeled available memory as "FREE" in the Dashboard System Overview for better clarity

### Changed
- **Aesthetic**: Standardized all history charts (CPU, Memory, GPU) to use high-fidelity Braille "dot" traces with simplified 0%/100% Y-axis labels for a cleaner, professional look
- **Visual Stability**: Fixed GPU chart X-axis to a stable 60-second window, preventing "squashing" as history fills up
- **Rebrand**: The project has been rebranded back to **PULS** from FOSPX SYSMON

### Fixed
- **Core Temperature Mapping**: Fixed core-to-sensor mapping for hyperthreaded systems, ensuring each logical thread reports accurate thermal data
- **GPU Layout**: Fixed overlapping widgets and incorrect indexing in the GPU detail panels

## [v0.8.1] - 2026-04-26

### Added
- **Hardware Limits**: Thermal limit readings added to the Sensors tab directly from `hwmon`
- **GPU Telemetry**: Added Memory Utilization, VRAM Temperature, and Fan RPM readouts for AMD, NVIDIA, and Intel architectures

### Changed
- **Rebrand**: The project was rebranded from PULS to FOSPX SYSMON (Reverted in v0.8.2)
- **Sensors UI**: Renamed the old "Limit" column to "Max Seen" and introduced a new dedicated "Limit" column for actual hardware maximums

### Removed
- **Dependencies Cleaned**: Removed some unnecessary dependencies to reduce binary bloat and improve compile times, but this also means that some planned changes will not be added, sorry :( 

## [v0.8.0] - 2026-02-26

### Added
- **Memory Tab**: Displays Memory Type, Generation, Speed (MT/s), and Temperature, may require sudo
- **Disks Tab**: Added Read and Write rate columns for individual disks
- **GPU Tab**: Restored Memory Clock and added PCIe version/width display

### Fixed
- **NVMe Detection**: Improved detection for NVMe health, power cycles, and temperature
- **AMD GPU Support**: Added more fallback paths for utilization and clock reporting
- **CPU Efficiency**: Normalized load average by core count for accurate efficiency ratings
- **Sensors Tab**: Fixed border color mismatch in dark themes

### Changed
- **CPU Cores Tab**: Fixed layout to 8 columns for better visibility
- **GPU Tab Layout**: Resized graphs to accommodate more hardware details

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
