# PULS

**A unified system monitoring and management tool for Linux.**

PULS combines resource monitoring with system administration capabilities. It allows control over system services, boot configurations, and logs directly from a TUI.

![PULS Screenshot](https://github.com/word-sys/puls/blob/main/screenshot.png)

## Architecture

PULS is built in Rust using `ratatui` for the interface and leverages native Linux APIs and binaries for system interaction:
*   **Monitoring**: Uses `sysinfo` for host metrics, `nvidia-smi` for NVIDIA GPUs, and a native DRM parser for AMD/Intel GPU telemetry. Supports multi-GPU configurations.
*   **System Control**: Interfaces directly with `systemd` (via `systemctl`) and `journald` (via `journalctl`) for service and log management.
*   **Process Management**: Advanced sorting logic including a "General" resource usage score combining CPU and Memory metrics.
*   **Configuration**: Parses and modifies `/etc/default/grub` and other system files with automatic backup generation.

## Features

### 1. Resource Monitoring
*   **CPU & Memory**: Per-core visualization and memory page breakdown.
*   **Disk I/O**: Read/Write monitoring per partition.
*   **Network**: Real-time upload/download rates for selected interfaces.
*   **NVIDIA, AMD & Intel GPUs**: Multi-vendor support with utilization, VRAM usage, temperature, and power telemetry. Visual history tracking included.

### 2. Process & Container Architecture
*   **Process Tree**: Sortable process list exposing PID, user, priority, and resource consumption.
*   **Container Engine Integration**: Connects to the local Docker socket to monitor container lifecycles, resource usage (CPU/Mem limits), and health status.

### 3. Service Management Subsystem
PULS provides control over `systemd` units:
*   **State Control**: Start, Stop, Restart services.
*   **Boot Persistence**: Enable or Disable services at startup.
*   **Status Inspection**: View full service definition and validation states.

### 4. Journal & Logging
*   **Aggregated Logs**: View `journald` logs directly within the TUI.
*   **Filtering**: Filter logs by specific system services, priority levels (Error/Warning), or specific boot sessions.

### 5. Boot Configuration (GRUB)
*   **Parameter Editing**: Modify kernel parameters in `/etc/default/grub`.
*   **Safety Backup**: PULS automatically creates a timestamped backup (e.g., `/etc/default/grub.bak.<timestamp>`) before applying any changes to boot configurations.

## Installation

### Static Binary (Portable)
The recommended way to run PULS on any Linux distribution (Debian, Fedora, Arch, Alpine) is using the statically linked MUSL binary. This avoids glibc version mismatches.

```bash
# 1. Download
wget -O puls https://github.com/word-sys/puls/releases/latest/download/puls

# 2. Verify and Install
chmod +x puls
sudo mv puls /usr/local/bin/puls
```

### Build from Source
To build the portable static binary:

1.  **Dependencies**:
    *   `musl-tools` (Debian/Ubuntu) or `musl-gcc` (Fedora) or `musl` (Arch).
    *   `rustup target add x86_64-unknown-linux-musl`

2.  **Build**:
    ```bash
    cargo build --release --target x86_64-unknown-linux-musl
    ```

### Build .deb Package
To create a Debian package compatible with older systems (Debian Buster/Bullseye, Ubuntu 20.04+):

1.  **Install cargo-deb**:
    ```bash
    cargo install cargo-deb
    ```

2.  **Build**:
    This uses the static `musl` build to ensure compatibility.
    ```bash
    rustup target add x86_64-unknown-linux-musl
    cargo deb --target x86_64-unknown-linux-musl
    ```
    The `.deb` file will be created in `target/x86_64-unknown-linux-musl/debian/`.

## Usage

PULS operates in different modes depending on the privileges and flags provided:

| Command | Capabilities |
| :--- | :--- |
| `puls` | **Read-only**: Monitoring of user processes, CPU/GPU, and Containers. |
| `sudo puls` | **Read/Write**: Full access to System Services (`systemctl`), Journals, and GRUB editing. |
| `puls --safe` | **Safety Mode**: Explicitly disables write capability, preventing accidental edits. |

---

*For release notes and updates, please visit the [GitHub Releases](https://github.com/word-sys/puls/releases) page.*
*Verified on Ubuntu 20.04+ and Arch Linux.*
