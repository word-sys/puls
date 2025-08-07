# PULS - A Modern Terminal Process Monitor

PULS is a responsive and feature-rich system monitoring dashboard that runs in your terminal. Its primary goal is to provide a clear, comprehensive, and interactive view of system processes, complemented by a high-level overview of hardware statistics.

Built with Rust, PULS allows you to quickly identify resource-intensive applications on the dashboard, and then instantly dive into a **Detailed Process View** to inspect the full command, user, environment variables, and more.

For reliability, PULS also features a **Safe Mode** (`--safe`), a lightweight diagnostic mode that ensures you can still analyze processes even when your system is under heavy load.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 
---

### Features

*   **Interactive Process List:** View a sortable list of running processes with details on PID, CPU usage, memory consumption, and disk activity.
*   **Detailed Process View:** Instantly inspect any selected process for its full command path, user, status, parent PID, start time, and all environment variables.
*   **Global Summary Bar:** An at-a-glance dashboard with live sparkline graphs for Network and Disk I/O, and gauges for overall CPU and Memory usage.
*   **Container & GPU Support:** Out-of-the-box monitoring for Docker/Podman containers and NVIDIA GPUs in its default mode.
*   **Safe Mode for Diagnostics:** A low-resource mode that disables I/O-heavy features to ensure stability during system emergencies.

---

## Installation (Linux)

The recommended way to install PULS is by downloading the latest pre-compiled binary using `wget`.

**1. Download & Install with one command:**
This command will download the latest `puls-linux` binary, make it executable, and move it to `/usr/local/bin/puls` so it can be run from anywhere. It may ask for your password for the final step.

```bash
wget -O puls-linux https://github.com/word-sys/puls/releases/latest/download/puls-linux && \
chmod +x puls-linux && \
sudo mv puls-linux /usr/local/bin/puls
```

**2. Run the Application:**
Now, open a new terminal and run the application in its two modes:

*   **Full-featured mode:**
    ```bash
    puls
    ```
*   **Low-resource safe mode:**
    ```bash
    puls --safe
    ```
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

**2. Build the optimized release:**
```bash
cargo build --release
```
The final executable will be located at `target/release/puls`. You can then install it using the `mv` command described in the Installation section above.

---

### Optional Dependencies (Full Mode Only)

For full functionality, the default mode of PULS relies on external services. These are not required for Safe Mode.

*   **Docker/Podman:** The Docker daemon must be running and accessible for container monitoring to function.
*   **NVIDIA GPU Support:** The official NVIDIA drivers must be installed, providing the `libnvidia-ml.so` library. If not found, the GPU sections will correctly show "N/A".

---

### Usage

*   **`q` or `Esc`:** Quit the application.
*   **`Tab` / `Backtab`:** Cycle through the different tabs.
*   **`1` - `7`:** Directly switch to a specific tab.
*   **`↑` / `↓`:** Select a process in the Dashboard view.
*   **`Enter`:** View detailed information for the selected process.
