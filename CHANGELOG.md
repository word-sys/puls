# Changelog

All notable changes to this project will be documented in this file.

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
