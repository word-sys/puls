# PULS - A Modern Terminal System Monitor

PULS is a fast, lightweight, and modern system monitoring tool that runs in your terminal. It is built with Rust and provides a comprehensive, at-a-glance overview of your system's key metrics, including CPU, GPU, memory, network, disk I/O, and processes.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 
---

### Features

*   **Global Summary Bar:** A graphical dashboard showing real-time usage for CPU, Memory, GPU, Network, and Disk I/O.
*   **Process Monitoring:** View a sortable list of running processes with details on PID, CPU usage, memory consumption, and disk activity.
*   **Detailed Process View:** Select any process to see its full command, user, status, start time, and environment variables.
*   **Container Support:** Automatically detects and displays statistics for running Docker/Podman containers, including CPU, memory, network, and disk usage.
*   **Hardware Details:** Individual tabs for viewing per-core CPU usage, disk partitions, network interface statistics, and detailed NVIDIA GPU stats (if available).
*   **Responsive UI:** Built with `ratatui` for a smooth and responsive terminal interface.

---

### Building from Source

You can build PULS yourself if you have the Rust toolchain installed.

**1. Clone the repository:**
```bash
git clone https://github.com/word-sys/puls.git
cd puls
```

**2. Build the project:**
For a regular debug build, run:
```bash
cargo build
```
For an optimized release build (recommended for installation), run:
```bash
cargo build --release
```
The final executable will be located at `target/release/puls`.

---

### Installation

You can install PULS system-wide to run it from anywhere by simply typing `puls`.

**Prerequisites:**
*   You have built the release binary as described above.

**Installation Command (Linux / macOS):**

This command will copy the optimized binary to `/usr/local/bin`, a standard location for user-installed executables. It will ask for your password as it requires administrator privileges.

```bash
# Run from the root of the project directory
sudo install target/release/puls /usr/local/bin/
```

After installing, open a new terminal session and run the application:
```bash
puls
```

### Usage

*   **`q` or `Esc`:** Quit the application.
*   **`Tab` / `Backtab`:** Cycle through the different tabs.
*   **`↑` / `↓`:** Select a process in the Dashboard view.
*   **`Enter`:** View detailed information for the selected process.
