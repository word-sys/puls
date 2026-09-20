**Languages:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="left">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="PULS Icon"/>
</p>

# PULS

**Ein einheitliches Systemüberwachungs- und Verwaltungswerkzeug für Linux**

PULS vereint präzise Ressourcenüberwachung mit nativen Linux-Systemadministrationsfunktionen. Es ermöglicht die Überwachung von Hardware-Telemetriedaten, die Verwaltung von systemd-Diensten, die Analyse von Systemprotokollen, die Inspektion hierarchischer Prozessbäume, das Senden von POSIX-Signalen, die sichere Bearbeitung von Boot-Konfigurationen und die Überwachung von Containern direkt über eine moderne, interaktive Terminal-Benutzeroberfläche (TUI).

## Bildschirmfotos

| **Dashboard & Systemübersicht** | **Hierarchischer Prozessbaum** |
| :---: | :---: |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png" alt="Dashboard & Systemübersicht" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png" alt="Hierarchischer Prozessbaum" width="450"/></a> |
| **CPU- & NUMA-Architektur** | **Speicher- & Swap-Aufteilung** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png" alt="CPU- & NUMA-Architektur" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png" alt="Speicher- & Swap-Aufteilung" width="450"/></a> |
| **Speicher, Inodes & Mount-Optionen** | **Netzwerkschnittstellen & Aktive Sockets** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png" alt="Speicher, Inodes & Mount-Optionen" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png" alt="Netzwerkschnittstellen & Sockets" width="450"/></a> |
| **Multi-GPU-Überwachung (NVIDIA & Intel)** | **Systeminfo, Sitzungen & Diagnose** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png" alt="Multi-GPU-Überwachung" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png" alt="Systeminfo & Diagnose" width="450"/></a> |
| **Systemd-Dienste & Timer** | **System-Journal-Logs & Boot-Verlauf** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png" alt="Systemd-Dienste & Timer" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png" alt="System-Journal-Logs" width="450"/></a> |
| **GRUB- & Bootloader-Konfiguration** | **Hardwaresensoren, Strom & Lüfter** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png" alt="GRUB-Konfiguration" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png" alt="Hardwaresensoren & Strom" width="450"/></a> |

---

## Architektur und Designprinzipien

PULS ist in Rust geschrieben und nutzt `ratatui` sowie `crossterm` für die Darstellung im Terminal. Es interagiert direkt mit dem Linux-Kernel und nativen Systemwerkzeugen:

*   **Keine neuen externen Abhängigkeiten**: Vermeidet unnötige Crates durch die ausschließliche Nutzung von Rust-Standardbibliotheksfunktionen, Unix-FFI-Aufrufen und direkten Kernel-Schnittstellen (`/proc`, `/sys`).
*   **Tiefe Hardware-Telemetrie**: Native Analyse von CPU-Topologien (L1/L2/L3-Cache, NUMA-Knoten-Affinitäten, Frequenz-Governors), Speicherseiten-Aufteilung, Dateisystem-Inodes (`statvfs`) und Batteriestatus (`/sys/class/power_supply/`).
*   **Multi-Vendor-GPU-Unterstützung**: Native Treiber-Parser über sysfs/DRM und hwmon für AMD- und Intel-Grafikchips sowie NVML/`nvidia-smi` für NVIDIA-GPUs. Überwacht VRAM-Auslastung, PCIe-Verbindungsgeneration und -breite, Lüfterdrehzahl (U/min), Leistungsaufnahme im Vergleich zu Leistungslimits und thermische Drosselung (Throttling).
*   **Transaktionaler Boot-Editor**: Bearbeiten Sie `/etc/default/grub` direkt im Arbeitsspeicher. Überprüfen Sie vor dem Speichern alle Änderungen in einem farblichen Diff-Dialog (`u`); automatische Backups mit Zeitstempel werden vor jedem Schreibvorgang erstellt.
*   **Robuste Terminalsteuerung**: Erfassung von Mausereignissen (`EnableMouseCapture`) für Klicks auf Tabs und flüssiges Scrollen; geschützt durch einen Panic-Hook, der den Terminalzustand beim Beenden stets sauber wiederherstellt.

---

## Funktionen

### 1. Ressourcen- und Hardwareüberwachung
*   **CPU und NUMA-Architektur**: Echtzeit-Auslastungsdiagramme pro Kern, anpassbares Rasterlayout für Prozessoren mit vielen Kernen (bis zu 128+ Kerne), L1/L2/L3-Cache-Größen, CPU-Governors sowie NUMA-Knoten-Zuordnung und Speicherverteilung.
*   **Arbeitsspeicher und Swap**: Detaillierte visuelle Aufschlüsselung von Gesamtspeicher, belegtem, freiem, verfügbarem, zwischengespeichertem Speicher und Swap. Unterstützt Temperatureinheiten in Celsius und Fahrenheit.
*   **Festplatten und Inodes**: Lese- und Schreibdurchsatzraten pro Partition, Speicherbelegung, Einhängeoptionen (Mount Flags) und prozentuale Inode-Belegung (`statvfs`).
*   **Netzwerk und Sockets**: Echtzeit-Übertragungsraten für Upload (`^`) und Download (`v`). Aktiver TCP/UDP-Socket-Inspektor mit Zuordnung von lokalen/entfernten IP-Endpunkten, Verbindungsstatus (ESTABLISHED, LISTEN usw.) und zugehörigen Prozess-PIDs.
*   **NVIDIA-, AMD- und Intel-GPUs**: Multi-GPU-Unterstützung mit Rechenauslastung, VRAM-Verbrauch, Taktraten, Temperaturen, Lüfterdrehzahlen (U/min), PCIe-Generierung/Breite, Leistungslimits und Drosselungsindikatoren.
*   **Stromversorgung und Akku**: Akkugesundheit, Ladeleistung in Watt, Ladezyklen, Kapazitätsbalken und Netzteil-Verbindungsstatus.

### 2. Prozess- und Containerverwaltung
*   **Hierarchischer Prozessbaum**: Umschalten zwischen flacher Liste und Eltern-Kind-Baumansicht (`t`) mit sauberen Baumglyphen (`├─`, `└─`).
*   **POSIX-Signal-Auswahl**: Senden Sie Unix-Signale direkt an Prozesse über ein modales Dialogfenster (`k` oder `F9`) mit Unterstützung für `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT` und `SIGINT`.
*   **Prozesspriorität anpassen (Renice)**: Priorität interaktiv verändern (`[` für höhere Priorität / niedrigeren Nice-Wert, `]` für niedrigere Priorität / höheren Nice-Wert).
*   **Detaillierter Prozess-Inspektor**: Drücken Sie `Eingabe` auf einem Prozess, um offene Dateideskriptoren (`/proc/[pid]/fd`), aktive Sockets, Thread-Anzahl und CPU-Auslastung einzelner Threads (`/proc/[pid]/task`) einzusehen.
*   **Interaktive Echtzeitsuche**: Drücken Sie `/` im Prozesse-Tab, um Prozesse sofort nach Namen zu filtern; `Esc` setzt den Filter zurück.
*   **Docker- und Podman-Container**: Verbindung zum Docker-Daemon oder zu Podman-Benutzersockets (`/run/user/$UID/podman/podman.sock`). Starten (`s`), Stoppen (`x`), Neustarten (`r`), Pausieren (`p`) und Live-Protokollanzeige (`Eingabe` oder `l`).

### 3. Systemd-Diensteverwaltung
Direkte Anbindung an `systemd` (`systemctl`):
*   **Statussteuerung**: Dienste Starten (`s`), Stoppen (`x`) und Neustarten (`r`).
*   **Autostart-Verwaltung**: Dienste beim Systemstart aktivieren (`e`) oder deaktivieren (`d`).
*   **Dienstinspektion**: Vollständige Unit-Definitionen, Abhängigkeiten und Gültigkeitsstatus einsehen.
*   **Protokollanzeige**: Drücken Sie `g`, um die letzten 50 `journald`-Einträge des ausgewählten Dienstes anzuzeigen.
*   **Geplante Timer**: Übersicht über aktive systemd-Timer (`systemctl list-timers`) mit nächstem und letztem Ausführungszeitpunkt.

### 4. Systemadministration und Diagnose
*   **Systemdiagnose**: Dashboard-Panel zur automatischen Erkennung von Systemanomalien wie hoher CPU-Temperatur, Speicherdruck oder kritischem Festplattenspeicher.
*   **Sitzungen und Neustartwarnung**: Aktive Benutzersitzungen (`who` / `/var/run/utmp`) und Benachrichtigung bei erforderlichem Systemneustart (`/var/run/reboot-required`).
*   **Zentrale Systemprotokolle**: Filtern Sie Protokolle nach Priorität (Fehler/Warnung), Dienst oder Boot-Sitzung. Schnelle Abfragen durch standardmäßige Beschränkung auf `--boot=0`.

### 5. Anpassung und Mehrsprachigkeit
*   **7 Sprachen unterstützt**: Vollständige Lokalisierung für Deutsch, Englisch (Standard), Türkisch, Französisch, Spanisch, Italienisch und Russisch. Automatische Erkennung über `LANG`/`LC_ALL` oder Umschalten zur Laufzeit (`L` / `l`).
*   **Einstellungsdialog (`F2` / `Shift+S`)**: Interaktives Konfigurationsmenü für Sprache, Farbschema, Standard-Start-Tab, Temperatureinheit (Celsius / Fahrenheit), Bildwiederholrate (500ms bis 5000ms) und Prozessbaumansicht. Gespeichert in `~/.config/puls/config.ini`.
*   **6 Farbthemen**: Standard, Dunkelblau, Hell, Dracula, Solarized Dark und Hoher Kontrast.
*   **Mausunterstützung**: Tabs per Klick wechseln, Einstellungen öffnen und Tabellen bequem mit dem Mausrad durchblättern.

---

## Tastenkürzel und Steuerung

### Globale Navigation
| Taste | Aktion |
| :--- | :--- |
| `q` / `Esc` | PULS beenden (oder aktiven Dialog schließen) |
| `Tab` / `1`..`9`, `0`, `-`, `=` | Tabs wechseln (0: Dashboard, 1: CPU, 2: Speicher, 3: Prozesse, 4: Festplatten, 5: Netzwerk, 6: GPU, 7: Sensoren, 8: Dienste, 9: Protokolle, 10: GRUB, 11: Container, 12: Systeminfo) |
| `Linksklick` | Tab-Kopfzeile anklicken oder Einstellungen-Schaltfläche betätigen |
| `F2` / `Shift+S` / `s` | Einstellungen-Dialog öffnen / schließen |
| `L` / `l` | Sprache durchschalten (DE -> ES -> IT -> RU -> EN -> TR -> FR) |
| `t` / `T` | Farbschema wechseln (außerhalb des Prozesse-Tabs) |
| `p` | Hintergrund-Metrikerfassung anhalten / fortsetzen |
| `?` | Hilfe- und Tastenkürzeldialog anzeigen |
| `Auf` / `Ab` / `j` / `k` / `Mausrad` | Durch Listen navigieren und Dialoginhalte scrollen |

### Prozesse-Tab (Tab 3)
| Taste | Aktion |
| :--- | :--- |
| `t` | Zwischen flacher Liste und hierarchischem Prozessbaum umschalten |
| `k` / `F9` | POSIX-Signal-Auswahl öffnen (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `[` | Prozesspriorität erhöhen (renice -1, höhere Priorität) |
| `]` | Prozesspriorität verringern (renice +1, niedrigere Priorität) |
| `Eingabe` | Detaillierten Prozess-Inspektor öffnen (Deskriptoren, Sockets, Threads) |
| `/` | Prozesse nach Name filtern |

### Dienste-Tab (Tab 8)
| Taste | Aktion |
| :--- | :--- |
| `s` | Ausgewählten Dienst starten |
| `x` | Ausgewählten Dienst stoppen |
| `r` | Ausgewählten Dienst neustarten |
| `e` | Dienst beim Systemstart aktivieren |
| `d` | Dienst beim Systemstart deaktivieren |
| `g` | Die letzten 50 journald-Zeilen des Dienstes anzeigen |

### Container-Tab (Tab 11)
| Taste | Aktion |
| :--- | :--- |
| `s` | Container starten |
| `x` | Container stoppen |
| `r` | Container neustarten |
| `p` | Container anhalten / fortsetzen |
| `Eingabe` / `l` | Live-Container-Protokolle öffnen |

### GRUB-Konfigurations-Tab (Tab 10)
| Taste | Aktion |
| :--- | :--- |
| `Eingabe` | Ausgewählten Konfigurationsparameter bearbeiten |
| `u` | Transaktionalen Diff-Vergleichsdialog vor dem Speichern öffnen |

---

## Installation

### Statische Binärdatei (Portabel)
Die empfohlene portable Installation nutzt die statisch gebundene MUSL-Binärdatei, die unabhängig von Bibliotheksversionen auf jeder Linux-Distribution lauffähig ist. Verfügbar für AMD64 (`x86_64`) und ARM64 (`aarch64`):

```bash
# Für x86_64 (AMD64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-x86_64-linux-musl.tar.gz
tar -xzf puls-0.9.4-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# Für aarch64 (ARM64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-aarch64-linux-musl.tar.gz
tar -xzf puls-0.9.4-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Debian / Ubuntu Paket (.deb)
Installieren Sie das offizielle `.deb`-Paket unter Debian, Ubuntu, Linux Mint oder Pop!_OS:

```bash
# AMD64-Paket herunterladen und installieren
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_amd64.deb
sudo apt install ./puls_0.9.4_amd64.deb

# Für ARM64-Systeme
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_arm64.deb
sudo apt install ./puls_0.9.4_arm64.deb
```

Falls Paketabhängigkeiten aufgelöst werden müssen:
```bash
sudo apt --fix-broken install
```

### Aus dem Quellcode bauen
So kompilieren Sie PULS lokal:

1. **Voraussetzungen**:
   * Rust-Toolchain (1.70+ empfohlen): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Musl-Compilerwerkzeuge: `sudo apt install musl-tools` (Debian/Ubuntu) oder `sudo dnf install musl-gcc` (Fedora) oder `sudo pacman -S musl` (Arch)
   * Musl-Ziel hinzufügen: `rustup target add x86_64-unknown-linux-musl`

2. **Kompilierung**:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   Die Binärdatei wird unter `target/x86_64-unknown-linux-musl/release/puls` erstellt.

---

## Nutzungsmodi

PULS passt seine Berechtigungen und Funktionen an die Ausführungsumgebung an:

| Befehl | Betriebsmodus |
| :--- | :--- |
| `puls` | **Benutzermodus**: Vollständige Überwachung von Benutzerprozessen, CPU, Speicher, Festplatten, Netzwerk, GPUs und Containern. |
| `sudo puls` | **Administratormodus**: Uneingeschränkter Zugriff auf systemd-Dienststeuerungen, Systemprotokolle und GRUB-Bearbeitung. |
| `puls --safe` | **Sicherer Modus**: Deaktiviert alle administrativen Schreibaktionen (Dienststeuerungen, Signalbefehle, GRUB-Speichern), um versehentliche Änderungen zu verhindern. |
| `puls --telemetry` | **Telemetriemodus**: Gibt detaillierte Messwerte zur Initialisierungsdauer beim Start auf der Standardausgabe aus. |

---

## Lizenz

PULS ist unter der [GNU General Public License v3.0](LICENSE) lizenziert.
