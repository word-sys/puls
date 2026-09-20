**Languages:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="center">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="Icona PULS"/>
</p>

# PULS

**Uno strumento unificato di monitoraggio e amministrazione del sistema per Linux**

PULS unisce il monitoraggio delle risorse ad alta fedeltà con funzionalità native di amministrazione del sistema Linux. Consente di monitorare la telemetria hardware, gestire i servizi systemd, consultare i registri journald, analizzare alberi di processi gerarchici, inviare segnali POSIX, modificare in sicurezza le configurazioni di boot e monitorare i container direttamente da un'interfaccia utente da terminale (TUI) interattiva e moderna.

![Schermata di PULS](https://raw.githubusercontent.com/word-sys/puls/main/screenshots/screenshot.png)

---

## Architettura e Principi di Progettazione

PULS è sviluppato in Rust, impiegando `ratatui` e `crossterm` per il rendering su terminale, e comunica direttamente con il kernel Linux e gli strumenti nativi di sistema:

*   **Zero Nuove Dipendenze**: Elimina librerie esterne superflue affidandosi alla libreria standard di Rust, chiamate FFI Unix personalizzate e ai filesystem virtuali del kernel (`/proc`, `/sys`).
*   **Telemetria Hardware Approfondita**: Analisi nativa della topologia della CPU (cache L1/L2/L3, affinità dei nodi NUMA, governor di frequenza), suddivisione delle pagine di memoria, inode del filesystem (`statvfs`) e telemetria della batteria (`/sys/class/power_supply/`).
*   **Supporto GPU Multi-Produttore**: Parser di driver sysfs/DRM e hwmon per schede grafiche AMD e Intel, integrati con query NVML/`nvidia-smi` per GPU NVIDIA. Traccia l'utilizzo della VRAM, la generazione e larghezza del collegamento PCIe, gli RPM delle ventole, il consumo energetico rispetto al limite e gli indicatori di throttling termico.
*   **Editor di Avvio Transazionale**: Modifica `/etc/default/grub` direttamente in memoria. Verifica le modifiche in una finestra di diff a colori (`u`) prima della scrittura su disco, con salvataggio automatico di backup contrassegnati da timestamp.
*   **Controllo Resiliente del Terminale**: Cattura degli eventi del mouse (`EnableMouseCapture`) per il cambio di scheda e lo scorrimento fluido delle tabelle, protetta da un hook di panico che ripristina sempre lo stato del terminale all'uscita.

---

## Funzionalità

### 1. Monitoraggio Risorse e Hardware
*   **CPU e Architettura NUMA**: Grafici a barre di utilizzo per singolo core in tempo reale, griglia adattiva per sistemi ad alto numero di core (fino a 128+ core), metriche di cache L1/L2/L3, governor di frequenza e affinità nodi/memoria NUMA.
*   **Memoria e Swap**: Ripartizione visiva dettagliata di memoria totale, utilizzata, libera, disponibile, in cache e swap. Supporto per le unità di temperatura in Celsius e Fahrenheit.
*   **Archiviazione e Inode**: Velocità di lettura/scrittura per partizione, spazio utilizzato, parametri di mount e percentuale di allocazione degli inode (`statvfs`).
*   **Rete e Socket**: Velocità di upload (`^`) e download (`v`) in tempo reale. Ispettore dei socket TCP/UDP attivi con associazione degli endpoint IP locali/remoti, stati di connessione (ESTABLISHED, LISTEN, ecc.) e PID dei processi proprietari.
*   **GPU NVIDIA, AMD e Intel**: Monitoraggio multi-GPU con carico di calcolo, utilizzo VRAM, frequenze di clock, temperature, velocità ventole (RPM), collegamento PCIe, limiti di alimentazione e stato di throttling.
*   **Alimentazione e Batteria**: Stato di salute della batteria, potenza di carica in Watt, conteggio cicli, percentuale e stato dell'adattatore di rete AC.

### 2. Gestione Processi e Container
*   **Albero Gerarchico dei Processi**: Alterna tra elenco lineare ordinabile e visualizzazione ad albero genitore-figlio (`t`) con glifi (`├─`, `└─`).
*   **Selettore Segnali POSIX**: Invia segnali Unix direttamente ai processi tramite una finestra modale (`k` o `F9`) che supporta `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT` e `SIGINT`.
*   **Modifica Priorità di Processo (Renice)**: Regola dinamicamente la priorità di esecuzione (`[` per aumentare la priorità / abbassare il valore nice, `]` per diminuire la priorità / alzare il valore nice).
*   **Ispettore Approfondito di Processo**: Premi `Invio` su qualsiasi processo per esaminare descrittori di file aperti (`/proc/[pid]/fd`), socket attivi, numero di thread e carico CPU per thread (`/proc/[pid]/task`).
*   **Ricerca Interattiva**: Premi `/` nella scheda Processi per filtrare in tempo reale per nome; `Esc` azzera il filtro.
*   **Container Docker e Podman**: Connessione al demone Docker o ai socket Podman utente (`/run/user/$UID/podman/podman.sock`). Avvia (`s`), Ferma (`x`), Riavvia (`r`), Sospendi (`p`) e visualizza i log in streaming dal vivo (`Invio` o `l`).

### 3. Sottosistema di Gestione Servizi
Integrazione diretta con `systemd` (`systemctl`):
*   **Controllo Stato**: Avvia (`s`), Ferma (`x`) e Riavvia (`r`) i servizi.
*   **Persistenza all'Avvio**: Abilita (`e`) o Disabilita (`d`) i servizi al boot.
*   **Ispezione del Servizio**: Visualizza la definizione completa dell'unità, le dipendenze e lo stato di convalida.
*   **Visualizzatore di Log**: Premi `g` per visualizzare le ultime 50 righe del registro `journald` del servizio selezionato.
*   **Timer Programmati**: Panoramica dei timer systemd attivi (`systemctl list-timers`) con le tempistiche delle prossime esecuzioni.

### 4. Amministrazione di Sistema e Diagnostica
*   **Diagnostica di Sistema**: Rilevamento anomalie nella dashboard per segnalare temperature CPU elevate, pressione della memoria o spazio disco critico.
*   **Sessioni Utente e Riavvio**: Sessioni utente attive (`who` / `/var/run/utmp`) e indicatore di riavvio di sistema richiesto (`/var/run/reboot-required`).
*   **Registri Journald Centralizzati**: Filtra i registri per priorità (Errore/Avviso), servizio o sessione di boot. Query istantanee limitate a `--boot=0`.

### 5. Personalizzazione e Internazionalizzazione
*   **7 Lingue Supportate**: Localizzazione completa per italiano, inglese (predefinito), turco, francese, tedesco, spagnolo e russo. Rilevamento automatico tramite `LANG`/`LC_ALL` o alternanza rapida (`L` / `l`).
*   **Finestra Impostazioni (`F2` / `Shift+S`)**: Configurazione interattiva per Lingua, Tema Colore, Scheda Predefinita all'avvio, Unità di Temperatura (Celsius / Fahrenheit), Frequenza di Aggiornamento (da 500ms a 5000ms) e visualizzazione albero processi. Salvataggio su `~/.config/puls/config.ini`.
*   **6 Temi Colore**: Predefinito, Blu Scuro, Chiaro, Dracula, Solarized Scuro e Contrasto Elevato.
*   **Supporto Mouse**: Clic per selezionare le schede e il pulsante Impostazioni; rotellina del mouse per lo scorrimento fluido di tabelle e log.

---

## Scorciatoie da Tastiera e Controlli

### Navigazione Globale
| Tasto | Azione |
| :--- | :--- |
| `q` / `Esc` | Esci da PULS (o chiudi il menu aperto) |
| `Tab` / `1`..`9`, `0`, `-`, `=` | Cambia scheda (0: Dashboard, 1: CPU, 2: Memoria, 3: Processi, 4: Dischi, 5: Rete, 6: GPU, 7: Sensori, 8: Servizi, 9: Registri, 10: GRUB, 11: Container, 12: Info Sistema) |
| `Clic Sinistro` | Fai clic sulla scheda nell'intestazione o sul badge Impostazioni |
| `F2` / `Shift+S` / `s` | Apri / Chiudi Finestra Impostazioni |
| `L` / `l` | Cambia lingua (IT -> RU -> EN -> TR -> FR -> DE -> ES) |
| `t` / `T` | Cambia tema colore (fuori dalla scheda Processi) |
| `p` | Metti in pausa / riprendi la raccolta delle metriche |
| `?` | Apri / Chiudi il menu di aiuto e comandi |
| `Su` / `Giù` / `j` / `k` / `Rotellina` | Scorri elementi degli elenchi e contenuti delle finestre modali |

### Scheda Processi (Scheda 3)
| Tasto | Azione |
| :--- | :--- |
| `t` | Alterna tra Elenco Lineare e Albero Gerarchico dei Processi |
| `k` / `F9` | Apri Selettore Segnali POSIX (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `[` | Aumenta priorità del processo (renice -1, priorità maggiore) |
| `]` | Diminuisci priorità del processo (renice +1, priorità minore) |
| `Invio` | Apri Ispettore Approfondito (descrittori, socket, thread) |
| `/` | Filtra processi per nome |

### Scheda Servizi (Scheda 8)
| Tasto | Azione |
| :--- | :--- |
| `s` | Avvia il servizio selezionato |
| `x` | Ferma il servizio selezionato |
| `r` | Riavvia il servizio selezionato |
| `e` | Abilita il servizio all'avvio |
| `d` | Disabilita il servizio all'avvio |
| `g` | Visualizza le ultime 50 righe di registro journald |

### Scheda Container (Scheda 11)
| Tasto | Azione |
| :--- | :--- |
| `s` | Avvia container |
| `x` | Ferma container |
| `r` | Riavvia container |
| `p` | Sospendi / Riprendi container |
| `Invio` / `l` | Apri visualizzatore di log del container dal vivo |

### Scheda Configurazione GRUB (Scheda 10)
| Tasto | Azione |
| :--- | :--- |
| `Invio` | Modifica parametro di configurazione selezionato |
| `u` | Apri confronto transazionale delle modifiche (diff) prima di salvare |

---

## Installazione

### Eseguibile Statico (Portabile)
L'installazione portabile consigliata utilizza il binario statico compilato con MUSL, senza dipendenze o requisiti specifici di glibc. Disponibile per AMD64 (`x86_64`) e ARM64 (`aarch64`):

```bash
# Per x86_64 (AMD64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-x86_64-linux-musl.tar.gz
tar -xzf puls-0.9.4-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# Per aarch64 (ARM64)
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-aarch64-linux-musl.tar.gz
tar -xzf puls-0.9.4-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Pacchetto Debian / Ubuntu (.deb)
Installa il pacchetto `.deb` ufficiale su Debian, Ubuntu, Linux Mint o Pop!_OS:

```bash
# Scarica e installa il pacchetto AMD64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_amd64.deb
sudo apt install ./puls_0.9.4_amd64.deb

# Per sistemi ARM64
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_arm64.deb
sudo apt install ./puls_0.9.4_arm64.deb
```

Nel caso sia necessario risolvere le dipendenze:
```bash
sudo apt --fix-broken install
```

### Compilazione da Sorgente
Per compilare PULS localmente:

1. **Prerequisiti**:
   * Toolchain Rust (1.70+ consigliato): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Strumenti di compilazione musl: `sudo apt install musl-tools` (Debian/Ubuntu) o `sudo dnf install musl-gcc` (Fedora) o `sudo pacman -S musl` (Arch)
   * Aggiungi target musl: `rustup target add x86_64-unknown-linux-musl`

2. **Compilazione**:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   L'eseguibile viene generato in `target/x86_64-unknown-linux-musl/release/puls`.

---

## Modalità d'Uso

PULS adatta le funzionalità in base ai permessi concessi:

| Comando | Modalità Operativa |
| :--- | :--- |
| `puls` | **Modalità Standard**: Monitoraggio completo di processi utente, CPU, memoria, dischi, rete, GPU e container. |
| `sudo puls` | **Modalità Amministratore**: Accesso completo ai controlli dei servizi systemd, registri e modifica di GRUB. |
| `puls --safe` | **Modalità Sicura**: Disabilita tutte le operazioni di scrittura (gestione servizi, segnali, salvataggio GRUB) per evitare modifiche involontarie. |
| `puls --telemetry` | **Modalità Telemetria**: Stampa su standard output i tempi di inizializzazione dettagliati durante l'avvio. |

---

## Licenza

PULS è distribuito sotto la licenza [GNU General Public License v3.0](LICENSE).
