# Changelog

All notable changes to this project will be documented in this file.

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
