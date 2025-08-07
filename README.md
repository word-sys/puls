# PULS - A Modern Terminal System Monitor

PULS is a responsive and feature-rich system monitoring dashboard that runs in your terminal. It is built with Rust and provides a comprehensive overview of your system's key metrics, including CPU, memory, network, disk I/O, processes, and container status.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 
---

### Features

*   **Global Summary Bar:** A graphical dashboard showing real-time usage for CPU, Memory, GPU, Network, and Disk I/O.
*   **Process Monitoring:** View a sortable list of running processes with details on PID, CPU usage, memory consumption, and disk activity.
*   **Detailed Process View:** Select any process to see its full command, user, status, start time, and environment variables.
*   **Container Support:** Automatically detects and displays statistics for running Docker/Podman containers.
*   **Hardware Details:** Individual tabs for viewing per-core CPU usage, disk partitions, network interface statistics, and detailed NVIDIA GPU stats.
*   **Cross-Platform:** Provides pre-compiled binaries for Linux, macOS, and Windows.

---

## Installation (Recommended)

The easiest way to install PULS is by downloading the latest pre-compiled binary for your operating system.

**1. Download the Latest Release:**
Go to the [**Releases Page**](https://github.com/word-sys/puls/releases/latest) on GitHub.

Download the appropriate binary for your system (e.g., `puls-linux`, `puls-macos`, or `puls-windows.exe`).

**2. Install the Binary:**

#### For Linux

Open your terminal, navigate to the directory where you downloaded the file, and run the following commands.

First, make the binary executable:
```bash
chmod +x puls-linux 
```

Next, move it to a location in your system's `PATH`. The recommended location is `/usr/local/bin`. This command will copy it there and may ask for your password.
```bash
sudo mv puls-linux /usr/local/bin/puls
```
*(Note: We rename it to just `puls` during the move for easier use.)*
---

## Building from Source

If you are a developer and prefer to compile the project yourself, you can build PULS from source.

**Prerequisites:**
*   [Rust Toolchain](https://www.rust-lang.org/tools/install) (rustc and cargo)

**1. Clone the repository:**
```bash
git clone https://github.com/word-sys/puls.git
cd puls
```

**2. Build the project:**
For an optimized release build, run:
```bash
cargo build --release
```
The final executable will be located at `target/release/puls`. You can then install it using the same steps outlined in the Installation section above.

---

### Optional Dependencies

For full functionality, PULS relies on external services and libraries being available on your system.

*   **Docker/Podman:** For container monitoring to function, the Docker daemon must be running and accessible to the user running PULS.
*   **NVIDIA GPU Support:** For GPU monitoring, the official NVIDIA drivers must be installed, which provide the `libnvidia-ml.so` library on Linux or `nvml.dll` on Windows. If these are not found, the GPU sections will correctly show "N/A".

---

### Usage

*   **`q` or `Esc`:** Quit the application.
*   **`Tab` / `Backtab`:** Cycle through the different tabs.
*   **`1` - `7`:** Directly switch to a specific tab (1=Dashboard, 2=Process, etc.).
*   **`↑` / `↓`:** Select a process in the Dashboard view.
*   **`Enter`:** View detailed information for the selected process.
