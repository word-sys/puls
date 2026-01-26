# PULS

A Rust-based detailed system monitoring and editing dashboard on TUI.

![PULS Screenshot](https://github.com/word-sys/puls/blob/main/screenshot.png) 


## What is PULS?

PULS provides an interactive and feature-rich system monitoring and editing session within your terminal. It provides real-time monitoring of processes with insightful information about CPU, memory, disk I/O, network also lets you edit your system services, edit GRUB and viewing system logs

## Key Features

- **Interactive Process List** - Sortable list of processes consuming resources
- **Detailed Process View** - Detailed view of process information, command lines, and environment variables
- **Container Monitoring** - Built-in Docker/Podman native container support
- **GPU Monitoring** - NVIDIA GPU support with real-time stats
- **Global Dashboard** - Live sparkline charts and system overview
- **Safe Mode** - Low-resource safe mode for system diagnostic
- **System Logs** - View the system logs of your system to diagnose any problems
- **GRUB Editing** - Edit your GRUB easily from PULS
- **System Services** - Add services, remove services, edit services, view services

## Installation

> [!CAUTION]
> This project is under development. There is some unconfirmed process that will be confirmed and updated in next updates: System service editing are existing and not existing, only reading them are working as which im confirmed, i didnt edited any system services using my tool so its not confirmed so its better to use on normal (without sudo), i will test the system services editing on spare computer, this is same for GRUB editing, viewing is confirmed but editing isnt, USE WITH CAUTION!

```bash
wget -O puls https://github.com/word-sys/puls/releases/latest/download/puls && \
chmod +x puls && \
sudo mv puls /usr/local/bin/puls
```

## Usage

```bash
sudo puls      # Full-featured mode (USE WITH CAUTION!)
puls           # Half-featured mode
puls --safe    # Safe mode (low resource usage)
```

## Build from Source (Portable)

To build a single binary that works on essentially all Linux distributions (including Debian 10, Ubuntu 20.04, and newer systems), follow these steps:

1.  **Install dependencies**:

    *   **Debian/Ubuntu**:
        ```bash
        sudo apt install musl-tools
        ```
    *   **Fedora**:
        ```bash
        sudo dnf install musl-gcc
        ```
    *   **Arch Linux**:
        ```bash
        sudo pacman -S musl
        ```

    Then add the Rust target:
    ```bash
    rustup target add x86_64-unknown-linux-musl
    ```

2.  **Build the project**:
    ```bash
    git clone https://github.com/word-sys/puls.git
    cd puls
    cargo build --release --target x86_64-unknown-linux-musl
    ```

The resulting binary will be located at `target/x86_64-unknown-linux-musl/release/puls`. This binary is statically linked and portable.

## Controls

- `q`/`Esc` - Quit
- `Tab` - Toggle tabs
- `↑↓` - Switch between processes
- `Enter` - Display process information
- `1-9,0,-,=` - Go to tab with specified number or character
- `p` - Pause/Resume process tab

## Requirements

- Linux environment
- Docker/Podman (optional, for monitoring containers)
- NVIDIA drivers (optional, for GPU monitoring)

---

*PULS is actively maintained. Go to the [releases page](https://github.com/word-sys/puls/releases) for the latest release.*
