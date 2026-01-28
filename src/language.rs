use std::fmt;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Turkish,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tr" | "turkish" => Language::Turkish,
            _ => Language::English,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::English => write!(f, "English"),
            Language::Turkish => write!(f, "Türkçe"),
        }
    }
}

pub struct Translator {
    lang: Language,
    en_dict: HashMap<&'static str, &'static str>,
    tr_dict: HashMap<&'static str, &'static str>,
}

impl Translator {
    pub fn new(lang: Language) -> Self {
        Self {
            lang,
            en_dict: Self::create_en_dict(),
            tr_dict: Self::create_tr_dict(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        match self.lang {
            Language::English => self.en_dict.get(key).unwrap_or(&key).to_string(),
            Language::Turkish => self.tr_dict.get(key).unwrap_or(&key).to_string(),
        }
    }

    fn create_en_dict() -> HashMap<&'static str, &'static str> {
        let mut map = HashMap::new();
        
        map.insert("tab.dashboard", "1:Dashboard");
        map.insert("tab.process", "2:Processes");
        map.insert("tab.cpu", "3:CPU");
        map.insert("tab.memory", "4:Memory");
        map.insert("tab.disks", "5:Disks");
        map.insert("tab.network", "6:Network");
        map.insert("tab.gpu", "7:GPU");
        map.insert("tab.system", "8:System");
        map.insert("tab.services", "9:Services");
        map.insert("tab.logs", "0:Logs");
        map.insert("tab.config", "-:Config");
        map.insert("tab.containers", "=:Docker");
        map.insert("title.config", "Configuration");
        map.insert("title.puls", "PULS - System Monitor & Admin Tool");
        map.insert("title.cpu", "CPU");
        map.insert("title.memory", "Memory");
        map.insert("title.gpu", "GPU");
        map.insert("title.network", "Network I/O");
        map.insert("title.disk", "Disk I/O");
        map.insert("title.processes", "Processes");
        map.insert("title.system_overview", "System Overview");
        map.insert("title.system_info", "System Information");
        map.insert("title.process_stats", "Process Statistics");
        map.insert("title.containers", "Containers");
        map.insert("title.cpu_cores", "CPU Cores");
        map.insert("title.disks", "Disks");
        map.insert("title.networks", "Networks");
        map.insert("title.gpus", "GPUs");
        map.insert("title.services", "System Services");
        map.insert("title.logs", "System Logs");
        map.insert("title.grub", "GRUB Settings");
        map.insert("title.network_config", "Network Configuration");
        map.insert("title.kernel", "Kernel Parameters");
        map.insert("header.pid", "PID");
        map.insert("header.name", "Name");
        map.insert("header.user", "User");
        map.insert("header.cpu", "CPU %");
        map.insert("header.memory", "Memory");
        map.insert("header.disk_read", "Disk Read");
        map.insert("header.disk_write", "Disk Write");
        map.insert("header.service", "Service");
        map.insert("header.status", "Status");
        map.insert("header.enabled", "Enabled");
        map.insert("header.uptime", "Uptime");
        map.insert("header.timestamp", "Timestamp");
        map.insert("header.level", "Level");
        map.insert("header.message", "Message");
        map.insert("status.running", "Running");
        map.insert("status.stopped", "Stopped");
        map.insert("status.failed", "Failed");
        map.insert("status.inactive", "Inactive");
        map.insert("status.activating", "Activating");
        map.insert("status.deactivating", "Deactivating");
        map.insert("status.enabled", "Enabled");
        map.insert("status.disabled", "Disabled");
        map.insert("status.active", "Active");
        map.insert("status.paused", "[PAUSED]");
        map.insert("health.idle", "IDLE");
        map.insert("health.normal", "NORMAL");
        map.insert("health.high", "HIGH");
        map.insert("health.overload", "OVERLOAD");
        map.insert("health.critical", "CRITICAL");
        map.insert("health.healthy", "HEALTHY");
        map.insert("health.moderate", "MODERATE");
        map.insert("alert.title", "ALERTS");
        map.insert("alert.high_cpu", "HIGH CPU!");
        map.insert("alert.high_memory", "HIGH MEMORY!");
        map.insert("alert.critical_memory", "CRITICAL MEMORY!");
        map.insert("alert.disk_critical", "DISK CRITICAL!");
        map.insert("alert.service_down", "SERVICE DOWN!");
        map.insert("help.main", "q:Quit | Tab/1-9:Navigate | ↑↓:Select | p:Pause | t:Theme | k:Kill | /:Search");
        map.insert("help.paused", "[PAUSED] Resume: p | Quit: q | Tabs: 1-9,0 | Navigate: ↑↓ | Details: Enter");
        map.insert("help.services", "↑↓: Navigate | Start: s | Stop: x | Restart: r | Enable: e | Disable: d | Edit: v | Quit: q");
        map.insert("help.logs", "↑↓: Navigate | Filter: f | Clear: c | Export: e | Search: / | Quit: q");
        map.insert("help.config", "↑↓: Navigate | Edit: e | Save: Ctrl+S | Revert: R | Quit: q");
        map.insert("log.debug", "DEBUG");
        map.insert("log.info", "INFO");
        map.insert("log.warning", "WARNING");
        map.insert("log.error", "ERROR");
        map.insert("log.critical", "CRITICAL");
        map.insert("action.start", "Start");
        map.insert("action.stop", "Stop");
        map.insert("action.restart", "Restart");
        map.insert("action.reload", "Reload");
        map.insert("action.enable", "Enable");
        map.insert("action.disable", "Disable");
        map.insert("action.status", "Status");
        map.insert("action.logs", "View Logs");
        map.insert("action.edit", "Edit");
        map.insert("action.save", "Save");
        map.insert("action.cancel", "Cancel");
        map.insert("info.hostname", "Hostname");
        map.insert("info.kernel", "Kernel");
        map.insert("info.uptime", "Uptime");
        map.insert("info.load", "Load Average");
        map.insert("info.cores", "CPU Cores");
        map.insert("info.threads", "Threads");
        map.insert("info.memory_total", "Total Memory");
        map.insert("info.memory_used", "Used Memory");
        map.insert("info.memory_available", "Available Memory");
        map.insert("info.swap_total", "Swap Total");
        map.insert("info.swap_used", "Swap Used");
        map.insert("config.grub_timeout", "GRUB Timeout");
        map.insert("config.grub_default", "GRUB Default Entry");
        map.insert("config.grub_cmd", "Kernel Command Line");
        map.insert("config.hostname", "Hostname");
        map.insert("config.timezone", "Timezone");
        map.insert("config.dns", "DNS Servers");
        map.insert("config.ntp", "NTP Service");
        map.insert("msg.success", "Success");
        map.insert("msg.error", "Error");
        map.insert("msg.warning", "Warning");
        map.insert("msg.confirm", "Confirm");
        map.insert("msg.loading", "Loading...");
        map.insert("msg.saved", "Configuration saved");
        map.insert("msg.failed", "Operation failed");
        map.insert("msg.unsaved", "Unsaved changes");
        map.insert("msg.container_disabled", "Container monitoring is disabled");
        map.insert("msg.no_containers", "No containers running");
        map.insert("status.sleeping", "Sleeping");
        map.insert("status.zombie", "Zombie");
        map.insert("status.other", "Other");
        map.insert("memory.healthy", "HEALTHY");
        map.insert("memory.moderate", "MODERATE");
        map.insert("memory.high", "HIGH");
        map.insert("memory.critical", "CRITICAL");
        map.insert("memory.comfortable", "COMFORTABLE");
        map.insert("memory.tight", "TIGHT");
        map.insert("efficiency.optimal", "OPTIMAL");
        map.insert("efficiency.good", "GOOD");
        map.insert("efficiency.fair", "FAIR");
        map.insert("efficiency.poor", "POOR");
        map.insert("label.load", "Load");
        map.insert("label.efficiency", "Eff");
        map.insert("label.available", "Available");
        map.insert("label.na", "N/A");
        map
    }

    fn create_tr_dict() -> HashMap<&'static str, &'static str> {
        let mut map = HashMap::new();
        
        map.insert("tab.dashboard", "1:Kontrol Paneli");
        map.insert("tab.process", "2:İşlemler");
        map.insert("tab.cpu", "3:CPU");
        map.insert("tab.memory", "4:Bellek");
        map.insert("tab.disks", "5:Diskler");
        map.insert("tab.network", "6:Ağ");
        map.insert("tab.gpu", "7:GPU");
        map.insert("tab.system", "8:Sistem");
        map.insert("tab.services", "9:Hizmetler");
        map.insert("tab.logs", "0:Günlükler");
        map.insert("tab.config", "-:Ayarlar");
        map.insert("tab.containers", "=:Konteynerler");
        map.insert("title.config", "Ayarlar");
        map.insert("title.puls", "PULS - Sistem İzleyici & Yönetim Aracı");
        map.insert("title.cpu", "CPU");
        map.insert("title.memory", "Bellek");
        map.insert("title.gpu", "GPU");
        map.insert("title.network", "Ağ G/Ç");
        map.insert("title.disk", "Disk G/Ç");
        map.insert("title.processes", "İşlemler");
        map.insert("title.system_overview", "Sistem Özeti");
        map.insert("title.system_info", "Sistem Bilgileri");
        map.insert("title.process_stats", "İşlem İstatistikleri");
        map.insert("title.containers", "Konteynerler");
        map.insert("title.cpu_cores", "CPU Çekirdekleri");
        map.insert("title.disks", "Diskler");
        map.insert("title.networks", "Ağ Arayüzleri");
        map.insert("title.gpus", "GPU'lar");
        map.insert("title.services", "Sistem Hizmetleri");
        map.insert("title.logs", "Sistem Günlükleri");
        map.insert("title.grub", "GRUB Ayarları");
        map.insert("title.network_config", "Ağ Yapılandırması");
        map.insert("title.kernel", "Çekirdek Parametreleri");
        map.insert("header.pid", "PID");
        map.insert("header.name", "Ad");
        map.insert("header.user", "Kullanıcı");
        map.insert("header.cpu", "CPU %");
        map.insert("header.memory", "Bellek");
        map.insert("header.disk_read", "Disk Okuma");
        map.insert("header.disk_write", "Disk Yazma");
        map.insert("header.service", "Hizmet");
        map.insert("header.status", "Durum");
        map.insert("header.enabled", "Etkin");
        map.insert("header.uptime", "Çalışma Süresi");
        map.insert("header.timestamp", "Zaman Damgası");
        map.insert("header.level", "Seviye");
        map.insert("header.message", "İleti");
        map.insert("status.running", "Çalışıyor");
        map.insert("status.stopped", "Durduruldu");
        map.insert("status.failed", "Başarısız");
        map.insert("status.inactive", "Pasif");
        map.insert("status.activating", "Başlatılıyor");
        map.insert("status.deactivating", "Durduruluyor");
        map.insert("status.enabled", "Etkin");
        map.insert("status.disabled", "Devre Dışı");
        map.insert("status.active", "Aktif");
        map.insert("status.paused", "[DURAKLATILDI]");
        map.insert("health.idle", "BOŞ");
        map.insert("health.normal", "NORMAL");
        map.insert("health.high", "YÜKSEK");
        map.insert("health.overload", "AŞIRI YÜK");
        map.insert("health.critical", "KRİTİK");
        map.insert("health.healthy", "SAĞLIKLI");
        map.insert("health.moderate", "UYGUN");
        map.insert("alert.title", "UYARILAR");
        map.insert("alert.high_cpu", "YÜKSEK CPU!");
        map.insert("alert.high_memory", "YÜKSEK BELLEK!");
        map.insert("alert.critical_memory", "KRİTİK BELLEK!");
        map.insert("alert.disk_critical", "DISK KRİTİK!");
        map.insert("alert.service_down", "HİZMET KAPALI!");
        map.insert("help.main", "q:Çık | Tab/1-9:Gezin | ↑↓:Seç | p:Duraklat | t:Tema | k:Sonlandır | /:Ara");
        map.insert("help.paused", "[DURAKLATILDI] Devam: p | Çık: q | Sekmeler: 1-9,0 | Gezin: ↑↓ | Detaylar: Enter");
        map.insert("help.services", "↑↓: Gezin | Başlat: s | Durdur: x | Yeniden Başlat: r | Etkinleştir: e | Devre Dışı: d | Düzenle: v | Çık: q");
        map.insert("help.logs", "↑↓: Gezin | Filtre: f | Temizle: c | Dışa Aktar: e | Ara: / | Çık: q");
        map.insert("help.config", "↑↓: Gezin | Düzenle: e | Kaydet: Ctrl+S | Geri Al: R | Çık: q");
        map.insert("log.debug", "HATA AYIKLAMA");
        map.insert("log.info", "BİLGİ");
        map.insert("log.warning", "UYARI");
        map.insert("log.error", "HATA");
        map.insert("log.critical", "KRİTİK");
        map.insert("action.start", "Başlat");
        map.insert("action.stop", "Durdur");
        map.insert("action.restart", "Yeniden Başlat");
        map.insert("action.reload", "Yeniden Yükle");
        map.insert("action.enable", "Etkinleştir");
        map.insert("action.disable", "Devre Dışı Bırak");
        map.insert("action.status", "Durum");
        map.insert("action.logs", "Günlükleri Görüntüle");
        map.insert("action.edit", "Düzenle");
        map.insert("action.save", "Kaydet");
        map.insert("action.cancel", "İptal");
        map.insert("info.hostname", "Bilgisayar Adı");
        map.insert("info.kernel", "Çekirdek");
        map.insert("info.uptime", "Çalışma Süresi");
        map.insert("info.load", "Ortalama Yük");
        map.insert("info.cores", "CPU Çekirdekleri");
        map.insert("info.threads", "Konular");
        map.insert("info.memory_total", "Toplam Bellek");
        map.insert("info.memory_used", "Kullanılan Bellek");
        map.insert("info.memory_available", "Kullanılabilir Bellek");
        map.insert("info.swap_total", "Toplam Takas");
        map.insert("info.swap_used", "Kullanılan Takas");
        map.insert("config.grub_timeout", "GRUB Zaman Aşımı");
        map.insert("config.grub_default", "GRUB Varsayılan Giriş");
        map.insert("config.grub_cmd", "Çekirdek Komut Satırı");
        map.insert("config.hostname", "Bilgisayar Adı");
        map.insert("config.timezone", "Saat Dilimi");
        map.insert("config.dns", "DNS Sunucuları");
        map.insert("config.ntp", "NTP Hizmeti");
        map.insert("msg.success", "Başarılı");
        map.insert("msg.error", "Hata");
        map.insert("msg.warning", "Uyarı");
        map.insert("msg.confirm", "Onayla");
        map.insert("msg.loading", "Yükleniyor...");
        map.insert("msg.saved", "Yapılandırma kaydedildi");
        map.insert("msg.failed", "İşlem başarısız");
        map.insert("msg.unsaved", "Kaydedilmemiş değişiklikler");
        map.insert("msg.container_disabled", "Konteyner izlemesi devre dışı bırakıldı");
        map.insert("msg.no_containers", "Çalışan konteyner yok");
        map.insert("status.sleeping", "Beklemede");
        map.insert("status.zombie", "Zombi");
        map.insert("status.other", "Diğer");
        map.insert("memory.healthy", "SAĞLIKLI");
        map.insert("memory.moderate", "UYGUN");
        map.insert("memory.high", "YÜKSEK");
        map.insert("memory.critical", "KRİTİK");
        map.insert("memory.comfortable", "RAHAT");
        map.insert("memory.tight", "SIKIŞTI");
        map.insert("efficiency.optimal", "OPTİMAL");
        map.insert("efficiency.good", "İYİ");
        map.insert("efficiency.fair", "ORTA");
        map.insert("efficiency.poor", "KÖTÜ");
        map.insert("label.load", "Yük");
        map.insert("label.efficiency", "Ver");
        map.insert("label.available", "Kullanılabilir");
        map.insert("label.na", "Yok");
        map
    }
}
