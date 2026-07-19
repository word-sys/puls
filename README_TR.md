![Language EN](https://github.com/word-sys/puls/)

<img src="https://raw.githubusercontent.com/word-sys/puls/main/puls_icon.svg" width="256" height="256" alt="PULS Icon"/>

# PULS

**Linux için birleşik bir sistem izleme ve yönetim aracı**

PULS, kaynak izlemeyi sistem yönetimi yetenekleriyle birleştirir. Sistem hizmetleri, önyükleme yapılandırmaları ve günlükleri doğrudan bir TUI'den kontrol etmenize olanak tanır ve ayrıca sistem sonuçlarınızı tek bir yerden izlemenizi sağlar.

![PULS Ekran Görüntüsü](https://raw.githubusercontent.com/word-sys/puls/main/screenshots/screenshot.png)

## Mimari

PULS, arayüz için `ratatui` kullanan ve sistem etkileşimi için yerel Linux API'lerinden ve ikili dosyalarından yararlanan Rust ile oluşturulmuştur:
*   **İzleme**: Ana bilgisayar metrikleri için `sysinfo`, NVIDIA GPU'lar için `nvidia-smi` ve AMD/Intel GPU telemetrisi için yerel bir DRM ayrıştırıcısı kullanır. Çoklu GPU yapılandırmalarını destekler.
*   **Sistem Kontrolü**: Servis ve günlük yönetimi için doğrudan `systemd` (`systemctl` aracılığıyla) ve `journald` (`journalctl` aracılığıyla) ile etkileşime girer.
*   **İşlem Yönetimi**: CPU ve Bellek kullanımını birleştiren bir "Genel" kaynak kullanım puanı da dahil olmak üzere gelişmiş sıralama mantığı.
*   **Yapılandırma**: `/etc/default/grub` ve diğer sistem dosyalarını yedek oluşturarak ayrıştırır ve değiştirir. Değişiklikler bellekte hazırlanır ve yalnızca açık onaydan sonra diske yazılır.


## Özellikler

### 1. Kaynak İzleme
*   **CPU ve Bellek**: L1/L2/L3 önbellek bilgileri ve bellek sayfası dökümü ile çekirdek başına görselleştirme.
*   **Disk G/Ç**: Bölüm başına Okuma/Yazma izleme.
*   **Ağ**: Seçilen arayüzler için gerçek zamanlı yükleme (`^`) / indirme (`v`) oranları.
*   **NVIDIA, AMD ve Intel GPU'lar**: Kullanım, VRAM kullanımı, sıcaklık ve güç telemetrisi ile çoklu üretici desteği. GPU özeti doğrudan Kontrol Panelinde gösterilir.

### 2. İşlem ve Konteyner Mimarisi
*   **İşlem Ağacı**: PID, kullanıcı, öncelik ve kaynak tüketimini gösteren sıralanabilir işlem listesi. Gerçek zamanlı olarak ada göre filtrelemek için `/` tuşuna basın.
*   **Konteyner Motoru Entegrasyonu**: Konteyner yaşam döngülerini, kaynak kullanımını (CPU/Bellek sınırları) ve sağlık durumunu izlemek için yerel Docker soketine bağlanır.

### 3. Hizmet Yönetim Alt Sistemi
PULS, `systemd` birimleri üzerinde kontrol sağlar:
*   **Durum Kontrolü**: Hizmetleri Başlatın, Durdurun, Yeniden Başlatın.
*   **Önyükleme Kalıcılığı**: Başlangıçta hizmetleri Etkinleştirin veya Devre Dışı Bırakın.
*   **Durum İncelemesi**: Tam hizmet tanımını ve doğrulama durumlarını görüntüleyin.
*   **Günlük Görüntüleyici**: Seçilen herhangi bir hizmet için son 50 `journald` günlük satırını görüntülemek için `g` tuşuna basın.

### 4. Günlük ve Kayıt (Journal)
*   **Birleştirilmiş Günlükler**: `journald` günlüklerini doğrudan TUI içinde görüntüleyin.
*   **Filtreleme**: Günlükleri belirli sistem hizmetlerine, öncelik seviyelerine (Hata/Uyarı) veya belirli önyükleme oturumlarına göre filtreleyin.

### 5. Önyükleme Yapılandırması (GRUB)
*   **İşlemsel Düzenleme**: GRUB parametrelerini bellekte düzenleyin. Değişiklikler diske yazılmadan önce tüm bekleyen değişikliklerin tam bir karşılaştırmasını (diff) gözden geçirmek için `u` tuşuna basın.
*   **Güvenlik Yedeği**: Değişiklikler uygulanmadan önce otomatik olarak zaman damgalı bir yedek (örneğin, `/etc/default/grub.bak.<zaman_damgası>`) oluşturulur.
*   **Sudo gerektirir**: Kök olmayan kullanıcılar için yazma işlemleri engellenir.

### 6. Tanılama
*   **Kontrol Paneli**: Yüksek CPU sıcaklığı, bellek baskısı ve kritik depolama alanı için satır içi anomali tespiti.
*   **Dil Algılama**: Başlangıçta `LANG`/`LC_ALL` ortam değişkenlerine göre otomatik olarak Türkçe veya İngilizceyi seçer.

## Kurulum

### Statik İkili Dosya (Taşınabilir)
PULS'u herhangi bir Linux dağıtımında (Debian, Fedora, Arch, Alpine) çalıştırmanın önerilen yolu, statik olarak bağlanmış MUSL ikili dosyasını kullanmaktır. Bu, glibc sürüm uyuşmazlıklarını önler.

```bash
wget -O puls https://github.com/word-sys/puls/releases/latest/download/puls
chmod +x puls
sudo mv puls /usr/local/bin/puls
```

### Debian Paketini Kurun (.deb)
Bu yöntem, Linux dağıtımları için en kolay kurulum yoludur.

1. [GitHub Releases](https://github.com/word-sys/puls/releases) sayfasından en son `.deb` paketini indirin. Dosya genellikle `puls_0.9.2-1_amd64.deb` gibi bir ada sahip olacaktır.

   > [!TIP] En kararlı deneyim için sürüm 0.9.2'yi kullanın: sürümler sayfasında `puls_0.9.2-1_amd64.deb` dosyasını arayın.

2. `.deb` dosyasını indirdiğiniz dizinde bir terminal açın.

3. Paketi kurmak için aşağıdaki komutu çalıştırın:

   ```bash
   sudo apt update
   sudo apt install ./puls_0.9.2-1_amd64.deb
   ```

   *(Not: Farklıysa, `puls_0.9.2-1_amd64.deb` dosyasını indirdiğiniz tam dosya adıyla değiştirin.)*

4. Kurulum sırasında bir bağımlılık hatasıyla karşılaşırsanız, eksik bağımlılıkları gidermek için aşağıdaki komutu çalıştırmayı deneyin:

   ```bash
   sudo apt --fix-broken install
   ```

5. Kurulum tamamlandığında, PULS'u uygulama menünüzden veya terminalinizden başlatabilirsiniz.

### Kaynaktan Derleme
Taşınabilir statik ikili dosyayı derlemek için:

1.  **Bağımlılıklar**:
    *   `musl-tools` (Debian/Ubuntu) veya `musl-gcc` (Fedora) or `musl` (Arch).
    *   `rustup target add x86_64-unknown-linux-musl`

2.  **Derleme**:
    ```bash
    cargo build --release --target x86_64-unknown-linux-musl
    ```

### .deb Paketi Derleme
Eski sistemlerle (Debian Buster/Bullseye, Ubuntu 20.04+) uyumlu bir Debian paketi oluşturmak için:

1.  **cargo-deb Kurulumu**:
    ```bash
    cargo install cargo-deb
    ```

2.  **Derleme**:
    Bu işlem, uyumluluğu sağlamak için statik `musl` derlemesini kullanır.
    ```bash
    rustup target add x86_64-unknown-linux-musl
    cargo deb --target x86_64-unknown-linux-musl
    ```
    `.deb` dosyası `target/x86_64-unknown-linux-musl/debian/` dizininde oluşturulacaktır.

## Kullanım

PULS, sağlanan yetkilere ve bayraklara bağlı olarak farklı modlarda çalışır:

| Komut | Yetenekler |
| :--- | :--- |
| `puls` | **Salt Okunur**: Kullanıcı işlemlerinin, CPU/GPU ve Konteynerlerin izlenmesi. |
| `sudo puls` | **Okuma/Yazma**: Sistem Hizmetlerine (`systemctl`), Günlüklere ve GRUB düzenlemeye tam erişim. |
| `puls --safe` | **Güvenli Mod**: Kazara düzenlemeleri önlemek için yazma özelliğini açıkça devre dışı bırakır. |

---

*Sürüm notları ve güncellemeler için lütfen [GitHub Releases](https://github.com/word-sys/puls/releases) sayfasını ziyaret edin.*
*Ubuntu 20.04+ ve Arch Linux üzerinde doğrulanmıştır.*
