**Diller:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="left">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="PULS Icon"/>
</p>

# PULS

**Linux için sıfır bağımlılıklı birleşik sistem izleme ve yönetim aracı**

PULS, yüksek hassasiyetli çekirdek telemetrisini yerel Linux sistem yönetimiyle **sıfır harici Rust paketi** (`[dependencies]` tamamen boş) içeren ~1.6 MB boyutunda tek bir statik ikili dosyada birleştirir. Donanım telemetrisini, Linux PSI basınç gecikmelerini, ZRAM ve disk I/O gecikmesini izleyin; sistem ve kullanıcı (`--user`) systemd hizmetlerini yönetin; yapılandırılmış journald günlüklerini inceleyin; hiyerarşik işlem ağaçlarını analiz edin; CPU yakınlığını ve I/O önceliğini ayarlayın; GRUB / `systemd-boot` / canlı çekirdek `sysctl` parametrelerini güvenle düzenleyin; Docker ve Podman konteynerlerini yönetin ve özel yüksek karşıtlıklı Terminal Kullanıcı Arayüzü (TUI) üzerinden telemetri oturumlarını kaydedip yeniden oynatın.

## Ekran Görüntüleri

| **Kontrol Paneli ve Sistem Özeti** | **Hiyerarşik İşlem Ağacı** |
| :---: | :---: |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png" alt="Kontrol Paneli ve Sistem Özeti" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png" alt="Hiyerarşik İşlem Ağacı" width="450"/></a> |
| **CPU ve NUMA Mimarisi** | **Bellek ve Takas (Swap) Tahsisi** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png" alt="CPU ve NUMA Mimarisi" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png" alt="Bellek ve Takas Tahsisi" width="450"/></a> |
| **Depolama, Inode'lar ve Bağlama Seçenekleri** | **Ağ Arayüzleri ve Etkin Soketler** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png" alt="Depolama, Inode'lar ve Bağlama Seçenekleri" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png" alt="Ağ Arayüzleri ve Etkin Soketler" width="450"/></a> |
| **Çoklu GPU İzleme (NVIDIA ve Intel)** | **Sistem Bilgisi, Oturumlar ve Tanılama** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png" alt="Çoklu GPU İzleme" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png" alt="Sistem Bilgisi ve Tanılama" width="450"/></a> |
| **Systemd Hizmetleri ve Zamanlayıcılar** | **Sistem Günlükleri ve Önyükleme Geçmişi** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png" alt="Systemd Hizmetleri ve Zamanlayıcılar" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png" alt="Sistem Günlükleri" width="450"/></a> |
| **GRUB, systemd-boot ve sysctl Ayarları** | **Donanım Sensörleri, Güç ve Fanlar** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png" alt="Önyükleyici ve sysctl Ayarları" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png" alt="Donanım Sensörleri ve Güç" width="450"/></a> |

---

## Mimari ve Tasarım İlkeleri

PULS v0.10.0, **sıfır harici paket bağımlılığı** ile tamamen Rust standart kütüphanesi ve doğrudan Linux çekirdek arayüzleri üzerine inşa edilmiştir:

*   **Sıfır Bağımlılık (`Cargo.toml` `[dependencies]` boş)**: `ratatui`, `crossterm`, `sysinfo`, `bollard`, `tokio`, `futures-util`, `log` ve `env_logger` tamamen kaldırılarak yerel Linux alt sistemleriyle değiştirildi (`Cargo.lock` 230 paketten 1'e düştü).
*   **Özel Linux Terminal Motoru (`src/term/`)**: Doğrudan `termios` ham modu, `ioctl(TIOCGWINSZ)` + `SIGWINCH` yeniden boyutlandırma, `poll(2)` engellemesiz girdi, UTF-8 klavye + SGR 1006 fare ayrıştırma, DEC Senkronize Çıktı destekli tam siyah sunucu ekranı çift tamponlu hücre fark oluşturucu ve 2×4 Unicode Braille grafik bileşenleri.
*   **Doğrudan Çekirdek ve Donanım Telemetrisi (`src/sys/`)**: Tek geçişli `/proc` ve `/sys` ayrıştırıcıları, ikili SMBIOS Type 17 DMI bellek tablosu çözümleyici, ikili `utmp`/logind oturum okuyucu, çalışma zamanı `dlopen("libnvidia-ml.so.1")` NVML (statik `musl` uyumlu), AMD/Intel için `/proc/[pid]/fdinfo` DRM VRAM eşlemesi ve doğrudan POSIX/Linux sistem çağrıları (`kill`, `setpriority`, `sethostname`, `sched_setaffinity`, `ioprio_set`).
*   **Yerel Konteyner İstemcisi**: Docker ve rootless Podman için Unix alan soketleri üzerinden özel HTTP/1.1 + JSON + çoklu günlük istemcisi ve Cgroup v2 hızlı telemetri yolu.
*   **İşlemsel Yapılandırma ve Önyükleme Düzenleyici**: GRUB, `systemd-boot` (`/boot/efi/loader/loader.conf`), `/etc/kernel/cmdline` ve canlı çekirdek `sysctl` parametrelerini bellekte düzenleyin; atomik olarak diske yazmadan önce renkli fark penceresinde (`u`) inceleyin.
*   **Dirençli Terminal Kontrolü**: Program kapanışında veya beklenmedik hatalarda ham modu, fare yakalamayı ve alternatif ekranı güvenle geri yükleyen panik kancası.

---

## Özellikler

### 1. Kaynak ve Donanım İzleme
*   **CPU, NUMA ve PSI**: Gerçek zamanlı çekirdek çubukları, 128+ çekirdek için uyarlanabilir ızgara düzenleri, L1/L2/L3 önbellek, frekans yöneticileri, NUMA düğüm yakınlığı ve Linux CPU Basınç Gecikmesi (`/proc/pressure/cpu`).
*   **Bellek, ZRAM ve PSI**: Yan yana RAM ve Takas göstergeleri, canlı Braille bellek geçmişi grafiği, görsel bellek bileşimi çubukları, ZRAM sıkıştırma oranı (`/sys/block/zram*/mm_stat`), SMBIOS Type 17 DIMM bilgileri ve Bellek PSI izleme.
*   **Depolama, Gecikme ve Inode'lar**: Çok panelli depolama ekranı, hacim kapasite kartları, `/proc/diskstats` `await_ms` ve `busy_pct` gecikmeleri, etkin I/O zamanlayıcı, SMART/NVMe sıcaklıkları, I/O PSI ve inode yüzdeleri (`statvfs`).
*   **Ağ ve Soket Denetleyici**: Arayüz bant genişliği, bağlantı hızı/MTU, `/proc/net/wireless` Wi-Fi dBm/kalite, `TxQ/RxQ` kuyrukları, TCP yeniden iletimleri, canlı `/` soket filtreleme ve sahip PID'ye `Enter` ile doğrudan geçiş.
*   **NVIDIA, AMD ve Intel GPU'lar**: Dinamik NVML ve yerel DRM/hwmon üzerinden çoklu GPU kullanımı, VRAM, saat hızları, sıcaklıklar, fan RPM, PCIe nesil/genişliği, güç sınırları, termal kısıtlama ve işlem başına VRAM kullanımı.
*   **Güç, Pil ve Sensörler**: Pil sağlığı, watt, voltaj, döngü sayısı, AC durumu ve gruplanmış hwmon sıcaklık/fan/voltaj/güç sensörleri.

### 2. İşlem ve Konteyner Yönetimi
*   **Hiyerarşik İşlem Ağacı ve Cgroup'lar**: Düz sıralanabilir liste ile ebeveyn-çocuk ağaç görünümü (`t`) arasında geçiş ve systemd birim / konteyner cgroup sütunu (`Unit / Cgroup`).
*   **Sinyaller, Renice, CPU Yakınlığı ve I/O Önceliği**: POSIX sinyalleri (`k`/`F9`), dinamik öncelik (`[`/`]`), etkileşimli CPU çekirdek maskesi (`a`, `sched_setaffinity`) ve I/O zamanlama önceliği (`i`, `ioprio_set`).
*   **Derinlemesine İşlem İnceleyici**: `/proc/[pid]/smaps_rollup` PSS/Paylaşılan/Özel/Takas bellek dökümü, açık dosya tanımlayıcıları/soketler, iş parçacığı başına CPU kullanımı ve ortam değişkenleri için `Enter` tuşuna basın.
*   **Docker ve Podman Konteynerleri**: Docker ve kullanıcı Podman soketlerini otomatik bulur. Başlatma (`s`), Durdurma (`x`), Yeniden Başlatma (`r`), Duraklatma (`p`) ve canlı günlük akışı (`Enter`/`l`).

### 3. Hizmet Yönetimi ve Yapılandırılmış Günlükler
*   **Sistem ve Kullanıcı (`--user`) Birimleri (`u`)**: `systemctl` ile root yetkisi gerektirmeyen `systemctl --user` arasında geçiş yapın; Başlat (`s`), Durdur (`x`), Yeniden Başlat (`r`), Etkinleştir (`e`), Devre Dışı Bırak (`d`), birim dosyasını görüntüle (`v`) ve son 50 günlüğü aç (`g`).
*   **Zamanlayıcılar ve Açılış Analizi (Boot Blame)**: `Sol`/`Sağ` ok tuşlarıyla zamanlayıcılar ve görsel açılış süresi çubukları (`Boot Blame`) arasında geçiş yapın.
*   **Yapılandırılmış Journald Günlükleri**: 4 sütunlu günlük görüntüleyici (`Zaman Damgası`, doğru JSON `PRIORITY` rozeti `EMERG..DEBUG`, `Hizmet / Birim`, `Mesaj`), canlı `/` filtreleme ve `<`/`>` önyükleme geçmişi geçişi.

### 4. Çoklu Önyükleyici, Canlı `sysctl` ve Anlık Görüntüler
*   **Önyükleyici ve Çekirdek `sysctl` Düzenleyici (Sekme 10)**: `Sol`/`Sağ` ile GRUB / `systemd-boot` / `/etc/kernel/cmdline` ve canlı `/proc/sys` çekirdek parametreleri (`/etc/sysctl.d/99-puls.conf` dosyasına atomik yazım) arasında geçiş yapın.
*   **Anlık Görüntü Dışa Aktarma ve Uçuş Kaydedici**: Başsız JSON/Markdown anlık görüntüsü (`--snapshot [YOL]` veya arayüzde `E`) ve çevrimdışı JSONL telemetri kaydı/oynatımı (`--record` / `--replay`, `<`/`>` ile zaman çizelgesinde gezinme).
*   **7 Dil ve 6 Tema**: Türkçe, English, Français, Deutsch, Español, Italiano, Русский (`L`/`l`), kalıcı Ayarlar penceresi (`F2`/`Shift+S` -> `~/.config/puls/config.ini`), 6 tema (`t`) ve tam SGR fare desteği.

---

## Kısayollar ve Kontroller

### Genel Gezinti
| Tuş | Eylem |
| :--- | :--- |
| `q` / `Esc` | PULS'tan çık (veya açık pencereyi kapat) |
| `Tab` / `1`..`9`, `0`, `-`, `=`, `+` | Sekmeler arasında geçiş (`1`: Kontrol Paneli, `2`: İşlemler, `3`: CPU, `4`: Bellek, `5`: Diskler, `6`: Ağ, `7`: GPU, `8`: Sistem, `9`: Hizmetler, `0`: Günlükler, `-`: Yapılandırma, `=`: Konteynerler, `+`: Sensörler) |
| `Sol Tıklama` / `Fare Tekerleği` | Sekmelere veya Ayarlar rozetine tıkla, tabloları ve pencereleri kaydır |
| `F2` / `Shift+S` / `s` | Ayarlar Yapılandırma Penceresini Aç / Kapat |
| `E` | Anlık JSON telemetri görüntüsü kaydet (`puls_snapshot_<zaman>.json`) |
| `<` / `>` | `--replay` modunda zaman çizelgesinde gezin (veya Günlükler sekmesinde önyükleme değiştir) |
| `L` / `l` | Arayüz dilini değiştir (TR -> FR -> DE -> ES -> IT -> RU -> EN) |
| `t` / `T` | Arayüz renk temasını değiştir (İşlemler sekmesi dışında) |
| `p` | Arka plan metrik toplamayı duraklat / devam ettir |

### İşlemler Sekmesi (`2`)
| Tuş | Eylem |
| :--- | :--- |
| `t` / `F5` | Düz Liste ile Hiyerarşik İşlem Ağacı Görünümü arasında geçiş yap |
| `k` / `F9` | POSIX Sinyal Seçici Penceresini Aç (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `a` | Etkileşimli CPU Yakınlık Maskesi Penceresini Aç (`sched_setaffinity`) |
| `i` | I/O Zamanlama Önceliği Penceresini Aç (`ionice` / `ioprio_set`) |
| `[` / `]` | İşlem önceliğini artır / azalt (`setpriority`) |
| `Enter` | Ayrıntılı İşlem İnceleyicisini Aç (Genel Bakış & PSS, FD/Soketler, İş Parçacıkları, Ortam) |
| `/` | İşlemleri ada göre filtrele |

### Ağ (`6`), Hizmetler (`9`), Yapılandırma (`-`) ve Konteynerler (`=`)
| Sekme ve Tuş | Eylem |
| :--- | :--- |
| Ağ `/` / `Enter` | Etkin soketleri filtrele (port/IP/durum/işlem) / Sahip PID'yi İşlem İnceleyicide aç |
| Hizmetler `Sol`/`Sağ` | Hizmetler, Zamanlayıcılar ve Boot Blame alt sekmeleri arasında geç |
| Hizmetler `u` / `v` / `g` | Sistem / `--user` kapsamını değiştir / Birim dosyasını gör / Son 50 journald günlüğünü aç |
| Hizmetler `s` / `x` / `r` / `e` / `d` | Seçili birimi Başlat / Durdur / Yeniden Başlat / Etkinleştir / Devre Dışı Bırak |
| Yapılandırma `Sol`/`Sağ` / `Enter` / `u` | Önyükleyici / Çekirdek `sysctl` geçişi / Değeri düzenle / Fark incele ve uygula |
| Konteynerler `s` / `x` / `r` / `p` / `l` | Başlat / Durdur / Yeniden Başlat / Duraklat / Canlı konteyner günlüklerini aç |

---

## Kurulum

### Statik İkili Dosya (Taşınabilir)
Önerilen taşınabilir kurulum, hiçbir çalışma zamanı bağımlılığı veya belirli bir glibc sürümü gerektirmeyen, statik olarak bağlanmış MUSL ikili dosyasını kullanır. AMD64 (`x86_64`) ve ARM64 (`aarch64`) için mevcuttur:

```bash
# x86_64 (AMD64) için
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.10.0-x86_64-linux-musl.tar.gz
tar -xzf puls-0.10.0-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# aarch64 (ARM64) için
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.10.0-aarch64-linux-musl.tar.gz
tar -xzf puls-0.10.0-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Debian / Ubuntu Paketi (.deb)
Debian, Ubuntu, Linux Mint veya Pop!_OS üzerine bağımsız `.deb` paketini kurun:

```bash
# AMD64 paketini indirin ve kurun
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.10.0_amd64.deb
sudo apt install ./puls_0.10.0_amd64.deb

# ARM64 sistemleri için
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.10.0_arm64.deb
sudo apt install ./puls_0.10.0_arm64.deb
```

Paket bağımlılıklarının çözülmesi gerekirse:
```bash
sudo apt --fix-broken install
```

### Kaynaktan Derleme
PULS **sıfır harici pakete** sahip olduğu için çevrimdışı ve kaynaktan derleme anlıktır:

1. **Ön Koşullar**:
   * Rust araç seti (1.75+ önerilir): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Statik derlemeler için isteğe bağlı musl araçları: `sudo apt install musl-tools` ve `rustup target add x86_64-unknown-linux-musl`

2. **Derleme**:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   İkili dosya `target/x86_64-unknown-linux-musl/release/puls` konumunda oluşturulur.

---

## Kullanım

PULS, verilen izinlere göre özelliklerini uyarlar:

| Komut | Çalışma Modu |
| :--- | :--- |
| `puls` | **Standart Mod**: İşlemlerin, CPU, bellek, diskler, ağ, GPU'lar, kullanıcı systemd birimlerinin ve konteynerlerin tam izlenmesi. |
| `sudo puls` | **Yönetici Modu**: Sistem geneli systemd kontrollerine, günlüklere, CPU/IO ayarına, önyükleyiciye ve `sysctl` düzenlemesine sınırsız erişim. |
| `puls --safe` | **Güvenli Mod**: Kazara yapılacak değişiklikleri önlemek için tüm yönetimsel yazma işlemlerini devre dışı bırakır. |
| `puls --snapshot [YOL]` | **Başsız Anlık Görüntü**: TTY gerektirmeden tam JSON sistem tanılamasını standart çıktıya yazdırır (veya `YOL` konumuna `.json`/`.md` kaydeder). |
| `puls --record <DOSYA.jsonl>` | **Uçuş Kaydedici**: TUI çalışırken canlı telemetri karelerini JSONL dosyasına kaydeder. |
| `puls --replay <DOSYA.jsonl>` | **Çevrimdışı Oynatma**: Kaydedilmiş JSONL uçuş günlüğünü `<`/`>` zaman çizelgesi gezintisiyle TUI içinde açar. |
| `puls --telemetry` | **Telemetri Modu**: Başlatma sırasında standart çıktıya tanısal açılış zamanlama ölçümlerini yazdırır. |

---

## Lisans

PULS, [GNU General Public License v3.0](LICENSE) altında lisanslanmıştır.
