# PULS

A terminal-based system monitoring dashboard built with Rust.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 

## What is PULS?

PULS provides a responsive and feature-rich system monitoring experience in your terminal. It offers real-time process monitoring with detailed insights into CPU, memory, disk I/O, and network usage.

## Key Features

- **Interactive Process List** - Sortable view of running processes with resource usage
- **Detailed Process View** - Deep dive into process details, command lines, and environment variables  
- **Container Monitoring** - Built-in support for Docker/Podman containers
- **GPU Monitoring** - NVIDIA GPU support with real-time metrics
- **Global Dashboard** - Live sparkline graphs and system overview
- **Safe Mode** - Low-resource mode for system diagnostics

## Installation

```bash
# Download and install latest release
wget -O puls-linux https://github.com/word-sys/puls/releases/latest/download/puls-linux && \
chmod +x puls-linux && \
sudo mv puls-linux /usr/local/bin/puls
```

## Usage

```bash
puls-linux           # Full-featured mode
puls-linux --safe    # Safe mode (low resource usage)
```

## Build from Source

```bash
git clone https://github.com/word-sys/puls.git
cd puls
cargo build --release
```

## Controls

- `q`/`Esc` - Quit
- `Tab` - Cycle tabs  
- `↑↓` - Navigate processes
- `Enter` - View process details
- `1-7` - Jump to specific tab
- `p` - Pause/Resume process tab

## Requirements

- Linux system
- Docker/Podman (optional, for container monitoring)
- NVIDIA drivers (optional, for GPU monitoring)

---

*PULS is actively developed. Check the [releases page](https://github.com/word-sys/puls/releases) for the latest updates.*
