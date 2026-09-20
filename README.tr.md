**Languages:** [English](README.md) | [Türkçe](README.tr.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Italiano](README.it.md) | [Русский](README.ru.md)

<p align="left">
  <img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="220" height="220" alt="PULS Simgesi"/>
</p>

# PULS

**Linux için birleşik bir sistem izleme ve yönetim aracı**

PULS, yüksek duyarlılıklı kaynak izlemeyi yerel Linux sistem yönetimi yetenekleriyle birleştirir. Donanım telemetrisini izlemenize, systemd hizmetlerini yönetmenize, journal günlüklerini incelemenize, hiyerarşik işlem ağaçlarını analiz etmenize, POSIX sinyalleri göndermenize, önyükleme yapılandırmalarını güvenle düzenlemenize ve konteyner yaşam döngülerini doğrudan etkileşimli bir Terminal Kullanıcı Arayüzünden (TUI) takip etmenize olanak tanır.

## Ekran Görüntüleri

| **Kontrol Paneli ve Sistem Genel Bakışı** | **Hiyerarşik İşlem Ağacı** |
| :---: | :---: |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot0.png" alt="Kontrol Paneli ve Sistem Genel Bakışı" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot1.png" alt="Hiyerarşik İşlem Ağacı" width="450"/></a> |
| **CPU ve NUMA Mimarisi** | **Bellek ve Takas Alanı Dağılımı** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot2.png" alt="CPU ve NUMA Mimarisi" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot3.png" alt="Bellek ve Takas Alanı" width="450"/></a> |
| **Depolama, Inode'lar ve Bağlama Seçenekleri** | **Ağ Arayüzleri ve Aktif Soketler** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot4.png" alt="Depolama ve Inode'lar" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot5.png" alt="Ağ Arayüzleri ve Soketler" width="450"/></a> |
| **Çoklu GPU İzleme (NVIDIA ve Intel)** | **Sistem Bilgisi, Oturumlar ve Tanılama** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot6.png" alt="Çoklu GPU İzleme" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot7.png" alt="Sistem Bilgisi ve Tanılama" width="450"/></a> |
| **Systemd Hizmetleri ve Zamanlayıcılar** | **Sistem Journal Günlükleri ve Önyükleme Geçmişi** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot8.png" alt="Systemd Hizmetleri" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot9.png" alt="Sistem Günlükleri" width="450"/></a> |
| **GRUB ve Önyükleyici Yapılandırması** | **Donanım Sensörleri, Güç ve Fanlar** |
| <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot10.png" alt="GRUB Yapılandırması" width="450"/></a> | <a href="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png"><img src="https://raw.githubusercontent.com/word-sys/puls/refs/heads/main/screenshots/screenshot11.png" alt="Donanım Sensörleri ve Güç" width="450"/></a> |

---

## Mimari ve Tasarım İlkeleri

PULS, kullanıcı arayüzü oluşturma için `ratatui` ve `crossterm` kullanarak Rust ile yazılmıştır; doğrudan Linux çekirdeği ve yerel sistem araçlarıyla haberleşir:

*   **Sıfır Yeni Bağımlılık**: Gereksiz harici paketleri ortadan kaldırır; standart kütüphane öğelerine, özel Unix FFI çağrılarına ve doğrudan çekirdek sanal dosya sistemlerine (`/proc`, `/sys`) dayanır.
*   **Derin Donanım Telemetrisi**: CPU topolojisi (L1/L2/L3 önbellek, NUMA düğüm yakınlıkları, frekans yöneticileri), bellek sayfası dökümü, dosya sistemi inode istatistikleri (`statvfs`) ve pil güç telemetrisi (`/sys/class/power_supply/`) için yerel çözümleme.
*   **Çoklu Üretici GPU Desteği**: AMD ve Intel grafikleri için yerel sysfs/DRM ve hwmon sürücü çözümleyicileri ile NVIDIA GPU'lar için NVML/`nvidia-smi` sorguları. VRAM kullanımını, PCIe bağlantı neslini ve genişliğini, fan devrini (RPM), güç sınırına karşı güç tüketimini ve donanım termal kısıtlama (throttling) bayraklarını izler.
*   **İşlemsel Önyükleme Düzenleyicisi**: `/etc/default/grub` dosyasını bellekte düzenleyin. Diske herhangi bir değişiklik yazılmadan önce renkli bir fark (diff) penceresinde (`u`) inceleyin; zaman damgalı anlık görüntülerle otomatik olarak yedeklenir.
*   **Dirençli Terminal Kontrolü**: Doğrudan sekme tıklamaları ve tablo kaydırma için terminal fare yakalama (`EnableMouseCapture`); program kapanışında terminal durumunu güvenle geri yükleyen bir panik kancası ile korunur.

---

## Özellikler

### 1. Kaynak ve Donanım İzleme
*   **CPU ve NUMA Mimarisi**: Gerçek zamanlı çekirdek başına kullanım çubukları, yüksek çekirdekli sistemler için uyarlanabilir ızgara düzenleri (128+ çekirdeğe kadar), L1/L2/L3 önbellek istatistikleri, frekans ölçeklendirme yöneticileri ve NUMA düğüm çekirdek yakınlığı ile bellek dağılımı.
*   **Bellek ve Takas (Swap)**: Toplam, kullanılan, boş, kullanılabilir, önbelleğe alınan ve takas arabelleklerinin görsel dökümü. Celsius ve Fahrenheit sıcaklık birimlerini destekler.
*   **Depolama ve Inode'lar**: Bölüm başına okuma/yazma aktarım hızları, depolama kullanımı, dosya sistemi bağlama seçenekleri ve inode tahsis yüzdeleri (`statvfs`).
*   **Ağ ve Soketler**: Gerçek zamanlı arayüz yükleme (`^`) ve indirme (`v`) bant genişliği hızları. Yerel/uzak IP uç noktalarını, bağlantı durumlarını (ESTABLISHED, LISTEN vb.) ve sahip olan işlem PID'lerini haritalayan etkin TCP/UDP soket denetleyicisi.
*   **NVIDIA, AMD ve Intel GPU'lar**: Hesaplama kullanımını, VRAM kullanımını, çekirdek/bellek saatlerini, sıcaklıkları, fan hızlarını (RPM), PCIe nesil/genişliğini, güç sınırlarını ve termal kısıtlama durumunu gösteren çoklu GPU izleme.
*   **Güç ve Pil**: Pil sağlığı, şarj watt değeri, döngü sayıları, yüzde çubukları ve AC adaptör bağlantı durumu.

### 2. İşlem ve Konteyner Yönetimi
*   **Hiyerarşik İşlem Ağacı**: Düz sıralanabilir liste ile ebeveyn-çocuk ağaç görünümü (`t`) arasında ağaç glifleri (`├─`, `└─`) ile geçiş yapın.
*   **POSIX Sinyal Seçici**: `SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT` ve `SIGINT` destekleyen modal (`k` veya `F9`) aracılığıyla işlemlere doğrudan Unix sinyalleri gönderin.
*   **İşlem Önceliği Ayarlama (Renice)**: İşlem zamanlama önceliğini dinamik olarak ayarlayın (`[` önceliği artırır / nice değerini düşürür, `]` önceliği azaltır / nice değerini yükseltir).
*   **Derinlemesine İşlem İnceleyici**: Açık dosya tanımlayıcı sayısını (`/proc/[pid]/fd`), etkin soket sayısını, iş parçacığı sayısını ve iş parçacığı düzeyinde CPU kullanımını (`/proc/[pid]/task`) görüntülemek için herhangi bir işlem üzerinde `Enter` tuşuna basın.
*   **Etkileşimli Arama**: Çalışan işlemleri ada göre gerçek zamanlı filtrelemek için İşlemler sekmesinde `/` tuşuna basın; `Esc` filtreyi temizler.
*   **Docker ve Podman Konteynerleri**: Docker daemon soketine veya kullanıcı Podman soketlerine (`/run/user/$UID/podman/podman.sock`) bağlanır. Konteynerleri başlatın (`s`), durdurun (`x`), yeniden başlatın (`r`), duraklatın (`p`) ve canlı konteyner günlüklerini izleyin (`Enter` veya `l`).

### 3. Hizmet Yönetim Alt Sistemi
`systemd` (`systemctl`) ile doğrudan entegrasyon:
*   **Durum Kontrolü**: Hizmetleri Başlatın (`s`), Durdurun (`x`) ve Yeniden Başlatın (`r`).
*   **Önyükleme Kalıcılığı**: Başlangıçta sistem hizmetlerini Etkinleştirin (`e`) veya Devre Dışı Bırakın (`d`).
*   **Hizmet Tanımı**: Tam systemd birim yapılandırmasını, bağımlılıklarını ve doğrulama durumlarını inceleyin.
*   **Günlük Görüntüleyici**: Seçilen hizmet için son 50 `journald` günlük girişini görüntülemek üzere `g` tuşuna basın.
*   **Zamanlanmış Zamanlayıcılar**: Bir sonraki tetikleme ve son tetikleme süreleriyle etkin systemd zamanlayıcılarının (`systemctl list-timers`) genel görünümü.

### 4. Sistem Yönetimi ve Tanılama
*   **Sistem Tanılaması**: Kontrol paneli anomali algılama, yüksek CPU sıcaklıklarını, bellek baskısını ve kritik disk kapasitesini vurgular.
*   **Önyükleme ve Kullanıcı Oturumları**: Etkin kullanıcı oturumları (`who` / `/var/run/utmp`) ve bekleyen sistem yeniden başlatma uyarıları (`/var/run/reboot-required`).
*   **Birleştirilmiş Journald Günlükleri**: Sistem günlüklerini önceliğe (Hata/Uyarı), birime veya önyükleme oturumuna göre filtreleyin. Anlık sorgular için `--boot=0` ile sınırlandırılmıştır.

### 5. Özelleştirme ve Çoklu Dil Desteği
*   **7 Desteklenen Dil**: Türkçe, English (varsayılan), Français, Deutsch, Español, Italiano ve Русский dilleri için tam yerelleştirme. `LANG`/`LC_ALL` ortam değişkenlerinden otomatik algılanır veya çalışma zamanında değiştirilebilir (`L` / `l`).
*   **Ayarlar Penceresi (`F2` / `Shift+S`)**: Arayüz Dili, Renk Teması, Varsayılan Başlangıç Sekmesi, Sıcaklık Birimleri (Celsius / Fahrenheit), Yenileme Hızı (500ms - 5000ms) ve Varsayılan İşlem Ağacı Görünümünü ayarlamak için etkileşimli yapılandırma penceresi. `~/.config/puls/config.ini` dosyasına kaydedilir.
*   **6 Seçkin Renk Teması**: Varsayılan, Koyu Mavi, Açık, Dracula, Solarized Dark ve Yüksek Karşıtlık.
*   **Fare Yakalama**: Fare ile sekme gezintisi, ayarlar iletişim kutusu geçişi ve yumuşak fare tekerleği ile kaydırma.

---

## Kısayollar ve Kontroller

### Genel Gezinti
| Tuş | Eylem |
| :--- | :--- |
| `q` / `Esc` | PULS'tan çık (veya açık olan pencereyi kapat) |
| `Tab` / `1`..`9`, `0`, `-`, `=` | Sekmeler arasında geçiş (0: Kontrol Paneli, 1: CPU, 2: Bellek, 3: İşlemler, 4: Diskler, 5: Ağ, 6: GPU, 7: Sensörler, 8: Hizmetler, 9: Günlükler, 10: GRUB, 11: Konteynerler, 12: Sistem Bilgisi) |
| `Sol Tıklama` | Gezinmek için sekme başlığına veya Ayarlar rozetine tıklayın |
| `F2` / `Shift+S` / `s` | Ayarlar Yapılandırma Penceresini Aç / Kapat |
| `L` / `l` | Arayüz dilini değiştir (TR -> FR -> DE -> ES -> IT -> RU -> EN) |
| `t` / `T` | Arayüz renk temasını değiştir (İşlemler sekmesi dışında) |
| `p` | Arka plan metrik toplamayı duraklat / devam ettir |
| `?` | Yardım ve Kısayollar Penceresini Aç / Kapat |
| `Yukarı` / `Aşağı` / `j` / `k` / `Fare Tekerleği` | Liste öğelerinde gezin ve modal içeriklerini kaydır |

### İşlemler Sekmesi (Sekme 3)
| Tuş | Eylem |
| :--- | :--- |
| `t` | Düz Liste ile Hiyerarşik İşlem Ağacı Görünümü arasında geçiş yap |
| `k` / `F9` | POSIX Sinyal Seçici Penceresini Aç (`SIGTERM`, `SIGKILL`, `SIGHUP`, `SIGSTOP`, `SIGCONT`, `SIGINT`) |
| `[` | İşlem önceliğini artır (renice -1, daha yüksek öncelik) |
| `]` | İşlem önceliğini azalt (renice +1, daha düşük öncelik) |
| `Enter` | Ayrıntılı İşlem İnceleyicisini Aç (Dosya Tanımlayıcıları, Soketler, İş Parçacıkları) |
| `/` | İşlemleri ada göre filtrele |

### Hizmetler Sekmesi (Sekme 8)
| Tuş | Eylem |
| :--- | :--- |
| `s` | Seçili hizmeti başlat |
| `x` | Seçili hizmeti durdur |
| `r` | Seçili hizmeti yeniden başlat |
| `e` | Seçili hizmeti başlangıçta etkinleştir |
| `d` | Seçili hizmeti başlangıçta devre dışı bırak |
| `g` | Hizmet için son 50 journald günlük satırını görüntüle |

### Konteynerler Sekmesi (Sekme 11)
| Tuş | Eylem |
| :--- | :--- |
| `s` | Konteyneri başlat |
| `x` | Konteyneri durdur |
| `r` | Konteyneri yeniden başlat |
| `p` | Konteyneri duraklat / devam ettir |
| `Enter` / `l` | Canlı konteyner günlükleri penceresini aç |

### GRUB Yapılandırma Sekmesi (Sekme 10)
| Tuş | Eylem |
| :--- | :--- |
| `Enter` | Seçili yapılandırma parametresini düzenle |
| `u` | Kaydetmeden önce işlemsel fark (diff) inceleme penceresini aç |

---

## Kurulum

### Statik İkili Dosya (Taşınabilir)
Önerilen taşınabilir kurulum, hiçbir çalışma zamanı bağımlılığı veya belirli bir glibc sürümü gerektirmeyen, statik olarak bağlanmış MUSL ikili dosyasını kullanır. AMD64 (`x86_64`) ve ARM64 (`aarch64`) için mevcuttur:

```bash
# x86_64 (AMD64) için
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-x86_64-linux-musl.tar.gz
tar -xzf puls-0.9.4-x86_64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls

# aarch64 (ARM64) için
wget https://github.com/word-sys/puls/releases/latest/download/puls-0.9.4-aarch64-linux-musl.tar.gz
tar -xzf puls-0.9.4-aarch64-linux-musl.tar.gz
sudo mv puls /usr/local/bin/puls
```

### Debian / Ubuntu Paketi (.deb)
Debian, Ubuntu, Linux Mint veya Pop!_OS üzerine bağımsız `.deb` paketini kurun:

```bash
# AMD64 paketini indirin ve kurun
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_amd64.deb
sudo apt install ./puls_0.9.4_amd64.deb

# ARM64 sistemleri için
wget https://github.com/word-sys/puls/releases/latest/download/puls_0.9.4_arm64.deb
sudo apt install ./puls_0.9.4_arm64.deb
```

Paket bağımlılıklarının çözülmesi gerekirse:
```bash
sudo apt --fix-broken install
```

### Kaynaktan Derleme
PULS'u yerel olarak derlemek için:

1. **Ön Koşullar**:
   * Rust araç seti (1.70+ önerilir): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   * Musl derleyici araçları: `sudo apt install musl-tools` (Debian/Ubuntu) veya `sudo dnf install musl-gcc` (Fedora) veya `sudo pacman -S musl` (Arch)
   * Musl hedefini ekleyin: `rustup target add x86_64-unknown-linux-musl`

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
| `puls` | **Standart Mod**: Kullanıcı işlemlerinin, CPU, bellek, diskler, ağ, GPU'lar ve konteynerlerin tam izlenmesi. |
| `sudo puls` | **Yönetici Modu**: Systemd hizmet kontrollerine, günlük kayıtlarına ve GRUB düzenlemesine sınırsız erişim. |
| `puls --safe` | **Güvenli Mod**: Kazara yapılacak değişiklikleri önlemek için tüm yönetimsel yazma işlemlerini (hizmet kontrolleri, işlem sonlandırma, GRUB kaydetme) devre dışı bırakır. |
| `puls --telemetry` | **Telemetri Modu**: Başlatma sırasında standart çıktıya tanısal açılış zamanlama ölçümlerini yazdırır. |

---

## Lisans

PULS, [GNU General Public License v3.0](LICENSE) altında lisanslanmıştır.
