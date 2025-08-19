# PULS

A Rust-based detailed system monitoring dashboard on TUI.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 


## What is PULS?

PULS provides an interactive and feature-rich system monitoring session within your terminal. It provides real-time monitoring of processes with insightful information about CPU, memory, disk I/O, and network.

## Key Features

- **Interactive Process List** - Sortable list of processes consuming resources
- **Detailed Process View** - Detailed view of process information, command lines, and environment variables
- **Container Monitoring** - Built-in Docker/Podman native container support
- **GPU Monitoring** - NVIDIA GPU support with real-time stats
- **Global Dashboard** - Live sparkline charts and system overview
- **Safe Mode** - Low-resource safe mode for system diagnostic

## Installation

> [!CAUTION]
> This project is under development. There is some minor bug(s) that will be fixed in next updates: CPU Mhz and Temps not showing, sometimes CPU Usage per process shows wrong values, I fixed this issue and tested it too but sometimes it happens, i will go deeper to find out why. I remade the hole calculation for this bug which still exists.

```bash
# Download and install latest release
wget -O puls-linux https://github.com/word-sys/puls/releases/latest/download/puls-linux && \
chmod +x puls-linux && \
sudo mv puls-linux /usr/local/bin/puls
```

## Usage

```bash
puls           # Full-featured mode
puls --safe    # Safe mode (low resource usage)
```

## Build from Source

```bash
git clone https://github.com/word-sys/puls.git
cd puls
cargo build --release
```

## Controls

- `q`/`Esc` - Quit
- `Tab` - Toggle tabs
- `↑↓` - Switch between processes
- `Enter` - Display process information
- `1-7` - Go to tab with specified number
- `p` - Pause/Resume process tab

## Requirements

- Linux environment
- Docker/Podman (optional, for monitoring containers)
- NVIDIA drivers (optional, for GPU monitoring)

---

*PULS is actively maintained. Go to the [releases page](https://github.com/word-sys/puls/releases) for the latest release.*
