# PULS - A Modern Terminal System Monitor

PULS is a responsive and feature-rich system monitoring dashboard that runs in your terminal. Built with Rust, it provides a comprehensive overview of your system's key metrics, including CPU, memory, network, disk I/O, processes, and container status.

A key feature of PULS is its **Safe Mode**, a low-resource diagnostic mode that disables I/O-heavy features like Docker and GPU monitoring. This allows PULS to run reliably on systems that are under heavy load or failing, making it a valuable tool for real-world problem-solving.

![PULS Screenshot](https://raw.githubusercontent.com/word-sys/puls/main/screenshot.png) 
---

### Features

*   **Dual-Mode Operation:**
    *   **Full Mode:** A rich, multi-threaded dashboard with live sparkline graphs, Docker/Podman integration, and NVIDIA GPU monitoring.
    *   **Safe Mode (`--safe`):** A lightweight, single-threaded diagnostic mode that prioritizes stability on low-resource systems.
*   **Global Summary Bar:** A graphical dashboard showing real-time usage for CPU, Memory, GPU, Network, and Disk I/O.
*   **Process Monitoring:** A sortable list of running processes with details on PID, CPU, memory, and disk activity.
*   **Detailed Process View:** Select any process to see its full command, user, status, start time, and environment variables.
*   **Optimized for Size:** Uses modern compiler features like LTO and symbol stripping to reduce binary size.

---

## Installation (Linux)

The easiest way to install PULS is by downloading the latest pre-compiled binary.

**1. Download the Latest Release:**
Go to the [**Releases Page**](https://github.com/word-sys/puls/releases/latest) on GitHub and download the `puls-linux` binary.

**2. Install the Binary:**
Open your terminal, navigate to your `Downloads` folder, and run the following commands.

First, make the binary executable:
```bash
chmod +x puls-linux ```

Next, move it to a standard system-wide location. This command renames the file to just `puls` and places it where it can be run from anywhere. It may ask for your password.
```bash
sudo mv puls-linux /usr/local/bin/puls
```

**3. Run the Application:**
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
The final executable will be located at `target/release/puls`. You can then install it using the same `mv` command described in the Installation section above.

---

### Optional Dependencies (Full Mode Only)

For full functionality, the default mode of PULS relies on external services. These are not required for Safe Mode.

*   **Docker/Podman:** The Docker daemon must be running and accessible for container monitoring to function.
*   **NVIDIA GPU Support:** The official NVIDIA drivers must be installed, providing the `libnvidia-ml.so` library. If not found, the GPU sections will show "N/A".

---

### Usage

*   **`q` or `Esc`:** Quit the application.
*   **`Tab` / `Backtab`:** Cycle through the different tabs.
*   **`1` - `7`:** Directly switch to a specific tab.
*   **`↑` / `↓`:** Select a process in the Dashboard view.
*   **`Enter`:** View detailed information for the selected process.
