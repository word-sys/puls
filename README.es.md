**Languages:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="center">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="Icono de PULS"/>
</p>

# PULS

**Una herramienta unificada de monitorización y administración del sistema para Linux**

PULS combina la monitorización de recursos de alta fidelidad con capacidades nativas de administración del sistema Linux. Permite monitorizar la telemetría del hardware, gestionar servicios de systemd, inspeccionar registros de journald, analizar árboles jerárquicos de procesos, emitir señales POSIX, modificar configuraciones de arranque de forma segura y supervisar contenedores directamente desde una interfaz de usuario en terminal (TUI) moderna e interactiva.

![Captura de pantalla de PULS](https://raw.githubusercontent.com/word-sys/puls/main/screenshots/screenshot.png)

---

## Arquitectura y Principios de Diseño

PULS está escrito en Rust y utiliza `ratatui` y `crossterm` para el renderizado visual en terminal, comunicándose directamente con el núcleo Linux y las herramientas nativas del sistema:

*   **Cero Nuevas Dependencias**: Elimina paquetes externos innecesarios apoyándose exclusivamente en la biblioteca estándar de Rust, llamadas FFI Unix personalizadas y los sistemas de archivos virtuales del núcleo (`/proc`, `/sys`).
*   **Telemetría Profunda de Hardware**: Análisis nativo de la topología de la CPU (caché L1/L2/L3, afinidades de nodos NUMA, gobernadores de frecuencia), desglose de páginas de memoria, inodos del sistema de archivos (`statvfs`) y telemetría de energía de la batería (`/sys/class/power_supply/`).
*   **Compatibilidad Multi-Fabricante con GPU**: Controladores sysfs/DRM y analizadores hwmon para gráficos AMD e Intel, junto con consultas directas NVML/`nvidia-smi` para GPU NVIDIA. Monitoriza uso de VRAM, generación y ancho de enlace PCIe, velocidad del ventilador (RPM), consumo frente a límites de potencia e indicadores de limitación térmica (throttling).
*   **Editor Transaccional de Arranque**: Modifique `/etc/default/grub` en memoria. Revise los cambios en un diálogo de comparación en color (`u`) antes de guardarlos en el disco, con copias de seguridad automáticas con marca de tiempo.
*   **Control Resistente del Terminal**: Captura de eventos del ratón (`EnableMouseCapture`) para navegación directa por pestañas y desplazamiento fluido de tablas, protegida por un gancho de pánico que restaura limpiamente el estado del terminal al salir.

---

## Características

### 1. Monitorización de Recursos y Hardware
*   **CPU y Arquitectura NUMA**: Gráficos de barras de uso por núcleo en tiempo real, diseño adaptable en cuadrícula para procesadores con gran cantidad de núcleos (hasta más de 128 núcleos), estadísticas de caché L1/L2/L3, gobernadores de escalado de frecuencia y distribución de afinidad de memoria/núcleos por nodo NUMA.
*   **Memoria y Swap**: Desglose visual detallado de memoria total, usada, libre, disponible, en caché y espacio swap. Admite unidades de temperatura en grados Celsius y Fahrenheit.
*   **Almacenamiento e Inodos**: Tasas de lectura y escritura por partición, espacio utilizado, indicadores de montaje y porcentajes de asignación de inodos (`statvfs`).
*   **Red y Sockets**: Velocidades de subida (`^`) y bajada (`v`) de interfaces en tiempo real. Inspector de sockets TCP/UDP activos que relaciona extremos IP locales y remotos, estados de conexión (ESTABLISHED, LISTEN, etc.) y los PID de los procesos propietarios.
*   **GPU NVIDIA, AMD e Intel**: Monitorización multi-GPU que muestra carga de cómputo, uso de VRAM, frecuencias de reloj, temperaturas, velocidades de ventiladores (RPM), enlaces PCIe, límites de consumo y alertas térmicas.
*   **Energía y Batería**: Salud de la batería, potencia de carga (Vatios), conteo de ciclos, barras de porcentaje y estado del adaptador de corriente alterna.

### 2. Gestión de Procesos y Contenedores
*   **Árbol Jerárquico de Procesos**: Alterne entre la lista plana ordenable y la vista de árbol jerárquico padre-hijo (`t`) con glifos de árbol (`├─`, `└─`).
*   **Selector de Señales POSIX**: Envíe señales Unix directamente a los procesos mediante un menú modal (`k` o `F9`) con soporte para `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT` y `SIGINT`.
*   **Ajuste de Prioridad de Procesos (Renice)**: Modifique la prioridad de planificación dinámicamente (`[` aumenta la prioridad / reduce el valor nice, `]` disminuye la prioridad / aumenta el valor nice).
*   **Inspector Profundo de Procesos**: Pulse `Intro` sobre cualquier proceso para inspeccionar descriptores de archivo abiertos (`/proc/[pid]/fd`), sockets activos, recuento de hilos y consumo de CPU por hilo (`/proc/[pid]/task`).
*   **Búsqueda Interactiva**: Pulse `/` en la pestaña Procesos para filtrar al instante por nombre; `Esc` restablece el filtro.
*   **Contenedores Docker y Podman**: Conexión con el demonio Docker o sockets Podman de usuario (`/run/user/$UID/podman/podman.sock`). Iniciar (`s`), Detener (`x`), Reiniciar (`r`), Pausar (`p`) y visualización en tiempo real de registros (`Intro` o `l`).

### 3. Subsistema de Administración de Servicios
Integración directa con `systemd` (`systemctl`):
*   **Control de Estado**: Iniciar (`s`), Detener (`x`) y Reiniciar (`r`) servicios.
*   **Persistencia en el Arranque**: Habilitar (`e`) o Deshabilitar (`d`) servicios al inicio del sistema.
*   **Inspección de Servicios**: Consulte la definición completa de la unidad systemd, dependencias y estados de validación.
*   **Visor de Registros**: Pulse `g` para consultar las últimas 50 líneas de registro de `journald` del servicio seleccionado.
*   **Temporizadores Programados**: Visión general de temporizadores activos de systemd (`systemctl list-timers`) con los tiempos de la próxima ejecución.

### 4. Administración del Sistema y Diagnóstico
*   **Diagnóstico del Sistema**: Detección de anomalías en el panel principal para alertar sobre temperaturas elevadas de la CPU, presión de memoria o almacenamiento crítico.
*   **Sesiones de Usuario y Reinicio**: Sesiones activas de usuarios (`who` / `/var/run/utmp`) y avisos de reinicio del sistema pendiente (`/var/run/reboot-required`).
*   **Registros Centralizados de Journald**: Filtre registros por prioridad (Error/Aviso), servicio o sesión de arranque. Consultas optimizadas limitadas a `--boot=0`.

### 5. Personalización e Internacionalización
*   **7 Idiomas Disponibles**: Localización completa para español, inglés (predeterminado), turco, francés, alemán, italiano y ruso. Detección automática mediante `LANG`/`LC_ALL` o cambio dinámico (`L` / `l`).
*   **Diálogo de Ajustes (`F2` / `Shift+S`)**: Configuración interactiva de idioma, tema de color, pestaña de inicio predeterminada, unidad de temperatura (Celsius / Fahrenheit), frecuencia de actualización (500 ms a 5000 ms) y vista de procesos. Guardado automático en `~/.config/puls/config.ini`.
*   **6 Temas de Color**: Predeterminado, Azul Oscuro, Claro, Dracula, Solarized Oscuro y Alto Contraste.
*   **Soporte de Ratón**: Clic para cambiar de pestaña y abrir ajustes; rueda del ratón para desplazamiento fluido en tablas y registros.

---

## Atajos de Teclado y Controles

### Navegación Global
| Tecla | Acción |
| :--- | :--- |
| `q` / `Esc` | Salir de PULS (o cerrar el diálogo abierto) |
| `Tab` / `1`..`9`, `0`, `-`, `=` | Cambiar de pestaña (0: Panel, 1: CPU, 2: Memoria, 3: Procesos, 4: Discos, 5: Red, 6: GPU, 7: Sensores, 8: Servicios, 9: Registros, 10: GRUB, 11: Contenedores, 12: Información del Sistema) |
| `Clic Izquierdo` | Hacer clic en la pestaña de encabezado o en el botón de Configuración |
| `F2` / `Shift+S` / `s` | Abrir / Cerrar Diálogo de Configuración |
| `L` / `l` | Alternar idioma (ES -> IT -> RU -> EN -> TR -> FR -> DE) |
| `t` / `T` | Cambiar tema de color (fuera de la pestaña Procesos) |
| `p` | Pausar / reanudar recolección de métricas |
| `?` | Abrir / Cerrar menú de ayuda y atajos |
| `Arriba` / `Abajo` / `j` / `k` / `Rueda` | Navegar por listas y desplazar contenido de ventanas modales |

### Pestaña Procesos (Pestaña 3)
| Tecla | Acción |
| :--- | :--- |
| `t` | Alternar entre Lista Plana y Vista Jerárquica de Árbol de Procesos |
| `k` / `F9` | Abrir Selector de Señales POSIX (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `[` | Aumentar prioridad del proceso (renice -1, mayor prioridad) |
| `]` | Disminuir prioridad del proceso (renice +1, menor prioridad) |
| `Intro` | Abrir Inspector Profundo de Procesos (descriptores, sockets, hilos) |
| `/` | Filtrar procesos por nombre |

### Pestaña Servicios (Pestaña 8)
| Tecla | Acción |
| :--- | :--- |
| `s` | Iniciar servicio seleccionado |
| `x` | Detener servicio seleccionado |
| `r` | Reiniciar servicio seleccionado |
| `e` | Habilitar servicio al inicio |
| `d` | Deshabilitar servicio al inicio |
| `g` | Ver las últimas 50 líneas del registro journald |

### Pestaña Contenedores (Pestaña 11)
| Tecla | Acción |
| :--- | :--- |
| `s` | Iniciar contenedor |
| `x` | Detener contenedor |
| `r` | Reiniciar contenedor |
| `p` | Pausar / Reanudar contenedor |
| `Intro` / `l` | Abrir visor de registros en vivo del contenedor |

### Pestaña Configuración GRUB (Pestaña 10)
| Tecla | Acción |
| :--- | :--- |
| `Intro` | Editar parámetro de configuración seleccionado |
| `u` | Abrir diálogo de revisión transaccional de diferencias (diff) antes de guardar |

---

## Instalación

### Binario Estático (Portátil)
La instalación portátil recomendada utiliza el binario estático compilado con MUSL, sin dependencias externas ni requisitos de versiones específicas de glibc. Disponible para AMD64 (`x86_64`) y ARM64 (`aarch64`):

```bash
# Para x86_64 (AMD64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-x86_64-linux-musl.tar.gz
tar -xzf puls-0.9.4-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# Para aarch64 (ARM64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-aarch64-linux-musl.tar.gz
tar -xzf puls-0.9.4-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Paquete Debian / Ubuntu (.deb)
Instale el paquete oficial `.deb` en Debian, Ubuntu, Linux Mint o Pop!_OS:

```bash
# Descargar e instalar el paquete para AMD64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_amd64.deb
sudo apt install ./puls_0.9.4_amd64.deb

# Para sistemas ARM64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_arm64.deb
sudo apt install ./puls_0.9.4_arm64.deb
```

Si necesita resolver dependencias del paquete:
```bash
sudo apt --fix-broken install
```

### Compilar desde el Código Fuente
Para compilar PULS localmente:

1. **Requisitos previos**:
   * Herramientas de Rust (1.70+ recomendado): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Herramientas de compilación musl: `sudo apt install musl-tools` (Debian/Ubuntu) o `sudo dnf install musl-gcc` (Fedora) o `sudo pacman -S musl` (Arch)
   * Añadir el objetivo musl: `rustup target add x86_64-unknown-linux-musl`

2. **Compilación**:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   El binario ejecutable se genera en `target/x86_64-unknown-linux-musl/release/puls`.

---

## Modos de Uso

PULS adapta sus características según los privilegios con los que se inicie:

| Comando | Modo Operativo |
| :--- | :--- |
| `puls` | **Modo Estándar**: Monitorización completa de procesos de usuario, CPU, memoria, discos, red, GPU y contenedores. |
| `sudo puls` | **Modo Administrador**: Acceso ilimitado a controles de servicios systemd, registros y modificación de GRUB. |
| `puls --safe` | **Modo Seguro**: Deshabilita todas las operaciones de escritura (controles de servicios, señales, guardado de GRUB) para evitar cambios accidentales. |
| `puls --telemetry` | **Modo Telemetría**: Imprime mediciones detalladas de los tiempos de inicio en la salida estándar durante la inicialización. |

---

## Licencia

PULS está publicado bajo la [Licencia Pública General GNU v3.0](LICENSE).
