<img src="https://raw.githubusercontent.com/fospx-org/fospx-sysmon/main/fospx-sysmon_icon.svg" width="256" height="256" alt="FOSPX SYSMON Icon"/>

# FOSPX SYSMON

**A unified system monitoring and management tool for Linux**

FOSPX SYSMON combines resource monitoring with system administration capabilities. It allows control over system services, boot configurations, and logs directly from a TUI also lets you monitor your system results everything in one place.

![FOSPX SYSMON Screenshot](https://raw.githubusercontent.com/fospx-org/fospx-sysmon/main/screenshots/screenshot.png)

## Architecture

FOSPX SYSMON built with Rust, using `ratatui` for the interface and leverages native Linux APIs and binaries for system interaction:
*   **Monitoring**: Uses `sysinfo` for host metrics, `nvidia-smi` for NVIDIA GPUs, and a native DRM parser for AMD/Intel GPU telemetry. Supports multi-GPU configurations.
*   **System Control**: Interfaces directly with `systemd` (via `systemctl`) and `journald` (via `journalctl`) for service and log management.
*   **Process Management**: Advanced sorting logic including a "General" resource usage score combining CPU and Memory usage.
*   **Configuration**: Parses and modifies `/etc/default/grub` and other system files with backup generation.

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
FOSPX SYSMON provides control over `systemd` units:
*   **State Control**: Start, Stop, Restart services.
*   **Boot Persistence**: Enable or Disable services at startup.
*   **Status Inspection**: View full service definition and validation states.

### 4. Journal & Logging
*   **Aggregated Logs**: View `journald` logs directly within the TUI.
*   **Filtering**: Filter logs by specific system services, priority levels (Error/Warning), or specific boot sessions.

### 5. Boot Configuration (GRUB)
*   **Parameter Editing**: Modify kernel parameters in `/etc/default/grub`.
*   **Safety Backup**: FOSPX SYSMON automatically creates a timestamped backup (e.g., `/etc/default/grub.bak.<timestamp>`) before applying any changes to boot configurations.

## Installation

### Static Binary (Portable)
The recommended way to run FOSPX SYSMON on any Linux distribution (Debian, Fedora, Arch, Alpine) is using the statically linked MUSL binary. This avoids glibc version mismatches.

```bash
wget -O fospx-sysmon https://github.com/fospx-org/fospx-sysmon/releases/latest/download/fospx-sysmon
chmod +x fospx-sysmon
sudo mv fospx-sysmon /usr/local/bin/fospx-sysmon
```

### Install Debian Package (.deb)
This method is the easiest installation path for Linux distributions.

1. Download the latest `.deb` package from the [GitHub Releases](https://github.com/fospx-org/fospx-sysmon/releases) page. The file will typically be named something like `fospx-sysmon_0.8.1-1_amd64.deb`.

   > [!TIP] Use version 0.8.1 for the most stable experience: look for `fospx-sysmon_0.8.1-1_amd64.deb` on the releases page.

2. Open a terminal in the directory where you downloaded the `.deb` file.

3. Run the following command to install the package:

   ```bash
   sudo apt update
   sudo apt install ./fospx-sysmon_0.8.1-1_amd64.deb
   ```

   *(Note: Replace `fospx-sysmon_0.8.1-1_amd64.deb` with the exact filename you downloaded if different.)*

4. If you encounter a dependency error during installation, try running the following command to fix missing dependencies:

   ```bash
   sudo apt --fix-broken install
   ```

5. Once installation is complete, you can launch FOSPX SYSMON from your application menu or terminal.

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

FOSPX SYSMON operates in different modes depending on the privileges and flags provided:

| Command | Capabilities |
| :--- | :--- |
| `fospx-sysmon` | **Read-only**: Monitoring of user processes, CPU/GPU, and Containers. |
| `sudo fospx-sysmon` | **Read/Write**: Full access to System Services (`systemctl`), Journals, and GRUB editing. |
| `fospx-sysmon --safe` | **Safety Mode**: Explicitly disables write capability, preventing accidental edits. |

---

*For release notes and updates, please visit the [GitHub Releases](https://github.com/fospx-org/fospx-sysmon/releases) page.*
*Verified on Ubuntu 20.04+ and Arch Linux.*
