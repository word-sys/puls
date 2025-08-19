# PULS - System Monitor v0.3.0

PULS is a responsive and feature-rich system monitoring dashboard that runs in your terminal. This version includes significant improvements, bug fixes, and new features over the original implementation.

## ✨ New Features in v0.3.0

### 🔧 Fixed Issues
- **Fixed CPU Usage Bug**: Corrected CPU usage calculation per process
- **Improved Performance**: 1-second refresh rate with smooth 60 FPS UI
- **Better Memory Management**: Reduced memory footprint and eliminated memory leaks

### 🚀 Enhanced Features
- **Advanced Process Details**: More comprehensive process information
- **Better Safe Mode**: Improved diagnostics and low-resource operation
- **Enhanced UI**: Smoother animations, better color schemes, responsive design
- **Improved Container Support**: Better Docker integration with more metrics
- **Network Monitoring**: Enhanced network interface monitoring
- **System Temperatures**: CPU and GPU temperature monitoring
- **Process Filtering**: Search and filter processes
- **Better Sorting**: Multiple sorting options for processes

### 🎨 UI Improvements
- Smooth 60 FPS rendering with 1-second data refresh
- Multiple color schemes (Dark, Light, Matrix, High Contrast, Solarized)
- Responsive design that adapts to terminal size
- Better error handling and status indicators
- Enhanced sparkline graphs with history
- Improved keyboard navigation

### ⚡ Performance Optimizations
- Async data collection with proper timeouts
- Efficient memory usage with history management
- Reduced CPU overhead for UI rendering
- Smart refresh rates (UI: 60 FPS, Data: 1 Hz)
- Optimized container monitoring

## 📋 Requirements

### System Requirements
- **OS**: Linux, macOS, Windows
- **Terminal**: 80x24 minimum (120x40 recommended)
- **Memory**: 50MB RAM minimum
- **CPU**: Any modern CPU

### Optional Dependencies
- **Docker**: For container monitoring
- **NVIDIA Drivers**: For NVIDIA GPU monitoring (libnvidia-ml.so)

## 🚀 Installation

### Option 1: Download Binary (Recommended)
```bash
# Download and install
wget -O puls-linux https://github.com/word-sys/puls/releases/latest/download/puls-linux
chmod +x puls-linux
sudo mv puls-linux /usr/local/bin/puls
```

### Option 2: Build from Source
```bash
# Clone repository
git clone https://github.com/word-sys/puls.git
cd puls

# Build release version
cargo build --release

# Install
sudo mv target/release/puls /usr/local/bin/puls
```

## 🎮 Usage

### Basic Usage
```bash
# Full-featured mode
puls

# Safe mode (limited features for diagnostics)
puls --safe

# Custom refresh rate (milliseconds)
puls --refresh 500

# Show system processes
puls --show-system

# Verbose mode
puls --verbose
```

### Command Line Options
- `--safe, -s`: Enable safe mode (disable Docker/GPU)
- `--refresh, -r <MS>`: Set refresh rate in milliseconds (default: 1000)
- `--history <COUNT>`: Set history length for graphs (default: 60)
- `--show-system`: Show system processes
- `--no-docker`: Disable Docker monitoring
- `--no-gpu`: Disable GPU monitoring  
- `--no-network`: Disable network monitoring
- `--auto-scroll`: Enable auto-scroll in process list
- `--verbose, -v`: Enable verbose logging

## 🎹 Keyboard Controls

### Navigation
- `q`, `Q`, `Esc`: Quit application
- `Tab` / `Shift+Tab`: Cycle through tabs
- `1`-`7`: Jump directly to tab
- `↑`/`↓`: Navigate process list
- `Enter`: View process details

### Controls
- `p`: Pause/resume data collection
- `Ctrl+C`: Sort by CPU usage
- `Ctrl+M`: Sort by memory usage  
- `Ctrl+N`: Sort by process name
- `Ctrl+S`: Toggle system processes
- `h`, `F1`: Show help (future feature)

## 📊 Tabs Overview

### 1. Dashboard
- **Top Panel**: Process list with CPU, memory, disk I/O
- **Bottom Panel**: Container list with resource usage
- Navigate with arrow keys, press Enter for details

### 2. Process Details  
- Complete process information
- Command line and environment variables
- Parent/child relationships
- Memory breakdown (RSS/VMS)

### 3. CPU Cores
- Individual core usage and frequency
- Temperature per core (if available)
- Visual gauges for each core

### 4. Disks
- Mount points and file systems
- Usage statistics and free space
- Disk I/O rates (if available)

### 5. Network
- Network interface statistics
- Real-time transfer rates
- Packet counts and error rates
- Interface status

### 6. GPU
- **NVIDIA**: Full GPU monitoring (utilization, memory, temperature, power)
- **AMD**: Future support planned
- **Intel**: Future support planned
- Multiple GPU support

### 7. System Info
- OS and kernel information
- Hardware specifications
- System uptime and load average
- Feature status

## 🔧 Configuration

### Feature Flags
Build with specific features:
```bash
# Full build with all features
cargo build --release --features full

# Docker only
cargo build --release --features docker

# GPU monitoring only  
cargo build --release --features nvidia-gpu

# Minimal build (no optional features)
cargo build --release --no-default-features
```

### Environment Variables
- `PULS_REFRESH_RATE`: Default refresh rate in milliseconds
- `PULS_SAFE_MODE`: Enable safe mode by default
- `RUST_LOG`: Set logging level (error, warn, info, debug, trace)

## 🎨 Themes and Colors

PULS includes multiple color schemes:
- **Dark**: Default dark theme
- **Light**: Light theme for bright terminals
- **Matrix**: Green Matrix-style theme
- **High Contrast**: Accessibility-focused theme
- **Solarized Dark**: Popular Solarized theme

Color-coded indicators:
- 🟢 **Green**: Good/Normal (0-50%)
- 🔵 **Blue**: Moderate (50-70%) 
- 🟡 **Yellow**: High (70-90%)
- 🟠 **Orange**: Very High (90-95%)
- 🔴 **Red**: Critical (95%+)

## 🛠️ Troubleshooting

### Common Issues

**Docker containers not showing**:
- Check if Docker daemon is running: `sudo systemctl status docker`
- Verify permissions: Add user to docker group
- Check Docker socket permissions

**GPU monitoring not working**:
- **NVIDIA**: Install official drivers and verify `nvidia-smi` works
- Run with `--verbose` for detailed error messages

**High CPU usage**:
- Try safe mode: `puls --safe`
- Increase refresh rate: `puls --refresh 2000`
- Disable expensive features: `puls --no-gpu --no-docker`

**Terminal too small**:
- Minimum: 80x24, recommended: 120x40
- Some features may be hidden in compact mode

### Performance Tuning

For low-end systems:
```bash
# Minimal resource usage
puls --safe --refresh 2000 --history 30
```

For high-end systems:
```bash
# Maximum features and responsiveness  
puls --refresh 250 --history 120 --show-system
```

## 🏗️ Architecture

### Modular Design
- **`monitors/`**: Data collection modules (system, GPU, containers)
- **`ui/`**: User interface components and rendering
- **`types.rs`**: Data structures and types
- **`utils.rs`**: Utility functions and formatters
- **`config.rs`**: Configuration management

### Performance Features
- Async data collection with timeouts
- Separate UI and data refresh rates
- Memory-efficient history buffers
- Smart update scheduling
- CPU usage optimization

### Thread Model
- **Main Thread**: UI rendering and event handling (60 FPS)
- **Data Collection Thread**: System monitoring (1 Hz)  
- **Async Tasks**: Container and GPU monitoring with timeouts

## 📝 License

GPL-3.0 License - see LICENSE file for details.

## 🔄 Changelog

### v0.3.0 (Current)
- Fixed CPU usage calculation bug
- Improved UI with 60 FPS rendering
- Enhanced process details and filtering
- Better error handling and diagnostics
- Optimized memory usage and performance
- Added multiple color schemes
- Improved safe mode functionality

### v0.2.0 (Previous)
- Basic container monitoring
- NVIDIA GPU support
- Core system monitoring
- Terminal UI with tabs

---

**Made with ❤️ by the PULS contributors**

For more information, visit: https://github.com/word-sys/puls