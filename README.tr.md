# skry

[English](README.md) · **Türkçe** · **[Web sitesi ve dokümantasyon](https://enderkus.github.io/skry/tr/)**

**Her sunucuyu gör. Hiçbir şey kurma.**

[![CI](https://github.com/enderkus/skry/actions/workflows/ci.yml/badge.svg)](https://github.com/enderkus/skry/actions/workflows/ci.yml)
[![Lisans: MIT VEYA Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#lisans)

skry, bir ya da birçok Linux sunucusunu **düz SSH üzerinden**, **uzak
makinelere hiçbir şey kurmadan ve yazmadan** izler. `/proc` dosyalarını ve
birkaç standart komutu okur; sonuçları hızlı bir terminal arayüzünde, isteğe
bağlı bir web panelinde, bir Prometheus uç noktasında ve betiklerde
kullanılabilir JSON olarak sunar.

![skry demo](docs/demo.gif)

- **Ajansız.** Servis yok, kopyalanan binary yok, geçici dosya yok, sudo yok.
- **Salt okunur.** skry uzak bir makinede hiçbir şeyi değiştirmez ve filoda
  keyfi komut çalıştırmanın hiçbir yolunu sunmaz.
- **Varsayılan olarak güvenli.** `known_hosts` dikkate alınır; `--accept-new`
  verilmedikçe bilinmeyen host anahtarları reddedilir. Parolalar asla saklanmaz.
- **Hafif.** Host başına tek SSH oturumu, her ölçümde tek bir toplu POSIX `sh`
  komutu.
- **Taşınabilir.** Debian, Ubuntu, RHEL/Rocky/Alma ve Alpine (BusyBox) üzerinde
  çalışır. Eksik ya da izin verilmeyen her şey `n/a` olarak görünür.
- Linux, macOS ve Windows için **tek binary**.

## İçindekiler

- [Neler sunar](#neler-sunar)
- [Kurulum](#kurulum)
- [Hızlı başlangıç](#hızlı-başlangıç)
- [Komut referansı](#komut-referansı)
- [Terminal arayüzü](#terminal-arayüzü)
- [Yapılandırma](#yapılandırma)
- [Web paneli ve Prometheus](#web-paneli-ve-prometheus)
- [Geçmiş, baseline'lar ve uyarılar](#geçmiş-baselinelar-ve-uyarılar)
- [Güvenlik modeli](#güvenlik-modeli)
- [Uzak makinede ne çalışır](#uzak-makinede-ne-çalışır)
- [Uyumluluk](#uyumluluk)
- [rtop'tan ilham](#rtoptan-ilham)
- [Lisans](#lisans)

## Neler sunar

| | |
| --- | --- |
| **Filo görünümü** | Sağlık durumuna göre renklenen (ok, warning, critical, unreachable, baseline sapması) host kutucuklarından oluşan bir ızgara; CPU, bellek, disk ve yük tek bakışta. Ada, gruba ya da duruma göre sıralama ve filtreleme. Kimlik doğrulama hataları, zaman aşımları ve host anahtarı sorunları kutucuğun üzerinde gösterilir; arızalı bir host diğerlerini asla bloklamaz. |
| **Host görünümü** | CPU (toplam ve çekirdek başına), bellek ve swap, yük, çalışma süresi, arayüz başına ağ hızları, cihaz başına disk kullanımı ve I/O, en yoğun süreçler, konteynerler (Docker/Podman), başarısız systemd birimleri, işletim sistemi ve çekirdek. |
| **Zamanda yolculuk** | Ölçümler yerel bir SQLite veritabanında tutulur (varsayılan 24 saat, otomatik olarak seyreltilir). Zaman çizelgesinde geriye giderek filoyu ya da bir hostu o anki hâliyle görebilirsiniz. |
| **Güvenlik nabzı** | Son 24 saatteki başarısız SSH girişleri, bekleyen güvenlik güncellemeleri (yalnızca önbellekteki paket meta verisinden), grup başına izin listesi dışında dinleyen portlar ve kendi makinenizden kontrol edilen TLS sertifikası bitiş tarihleri. |
| **Baseline'lar** | Host ve metrik başına, isteğe bağlı olarak günün her saati için ayrı tutulan ortalama ve varyansın EWMA'sı. Sabit eşiklerin yanında, *o host için* olağandışı olan değerler de işaretlenir. |
| **Uyarılar** | Eşik aşımları, baseline sapmaları, erişilemeyen hostlar ve güvenlik bulguları için Slack, Discord ve genel JSON webhook'ları; tekilleştirme ve bekleme süresi (cooldown) ile. |
| **Olay anlık görüntüleri** | `s` tuşuna basarak (ya da `skry snapshot` çalıştırarak) seçili hostların tam durumunu Markdown ve JSON olarak kaydedin; isteğe bağlı olarak bir webhook'a gönderin. |
| **Web + Prometheus** | `skry serve`, canlı güncellenen salt okunur bir panel ve bir `/metrics` uç noktası sunar. |

## Kurulum

skry kendi iş istasyonunuzda (ya da bir atlama sunucusunda) çalışır. İzlenen
hostlarda yalnızca bir SSH sunucusu ve bir POSIX kabuğu gerekir.

### Hızlı kurulum

**Linux ve macOS**

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | sh
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

Kurulum betiği platformunuzu algılar, en son sürümü indirir, SHA-256
checksum'ını doğrular ve binary'yi kurar:

| Platform | Kurulum yeri |
| --- | --- |
| Linux, macOS | Yazılabilirse `/usr/local/bin`, değilse `~/.local/bin` |
| Windows | `%LOCALAPPDATA%\Programs\skry`; kullanıcı `PATH`'ine eklenir |

İki isteğe bağlı ortam değişkeni varsayılanları değiştirir:

```sh
# Belirli bir sürüm, seçtiğiniz bir dizine
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | SKRY_VERSION=v0.1.0 SKRY_INSTALL_DIR=/opt/bin sh
```

```powershell
$env:SKRY_VERSION = 'v0.1.0'; irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

Bir betiği çalıştırmadan önce okumayı mı tercih edersiniz?
[`install.sh`](install.sh) ya da [`install.ps1`](install.ps1) dosyasını
indirin, göz atın ve yerelde çalıştırın.

### Elle indirme (v0.1.0)

| Platform | Arşiv | Checksum |
| --- | --- | --- |
| Linux x86_64 (statik) | [skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256) |
| Linux aarch64 (statik) | [skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz.sha256) |
| macOS Apple Silicon | [skry-v0.1.0-aarch64-apple-darwin.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256) |
| macOS Intel | [skry-v0.1.0-x86_64-apple-darwin.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-apple-darwin.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-apple-darwin.tar.gz.sha256) |
| Windows x86_64 | [skry-v0.1.0-x86_64-pc-windows-msvc.zip](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-pc-windows-msvc.zip) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-pc-windows-msvc.zip.sha256) |

Daha yeni sürümler [sürümler sayfasında](https://github.com/enderkus/skry/releases) listelenir.

**Linux** (ARM'da `x86_64` yerine `aarch64` kullanın)

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
sha256sum -c skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
tar xzf skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
sudo install skry-v0.1.0-x86_64-unknown-linux-musl/skry /usr/local/bin/
```

**macOS** (Intel'de `aarch64` yerine `x86_64` kullanın)

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
shasum -a 256 -c skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar xzf skry-v0.1.0-aarch64-apple-darwin.tar.gz
sudo install skry-v0.1.0-aarch64-apple-darwin/skry /usr/local/bin/
```

Tarayıcıyla indirilen arşivler Gatekeeper tarafından karantinaya alınır;
işareti bir kez `xattr -d com.apple.quarantine /usr/local/bin/skry` ile
kaldırın.

**Windows**

Zip'i indirin, açın ve `skry.exe`'yi `PATH`'inizdeki bir klasöre taşıyın.
skry, Windows OpenSSH ajanıyla (`ssh-agent` servisi) ya da Pageant ile konuşur;
`%USERPROFILE%\.ssh\config` ve `known_hosts` dosyalarını okur. En iyi arayüz
görüntüsü için Windows Terminal kullanın.

### Kaynaktan

Güncel bir kararlı Rust araç zinciriyle (1.85 veya üstü):

```sh
cargo install --locked --git https://github.com/enderkus/skry
```

### Kaldırma

Binary'yi silin (`/usr/local/bin/skry`, `~/.local/bin/skry` ya da
Windows'ta `%LOCALAPPDATA%\Programs\skry`; Windows'ta kullanıcı `PATH`'indeki
girdiyi de kaldırabilirsiniz). skry, kullandıysanız yapılandırma dosyası ve
geçmiş veritabanı dışında hiçbir şey bırakmaz (`skry config path`
yapılandırmanın yerini gösterir; geçmiş platformun veri dizinindedir, örneğin
Linux'ta `~/.local/share/skry`, macOS'ta `~/Library/Application Support/skry`).

## Hızlı başlangıç

```sh
# ~/.ssh/config içindeki bir host
skry web1

# Birden çok host; takma adlar ile açık user@host:port bir arada
skry web1 web2 deploy@10.0.0.7:2222

# Yeni hostlarla ilk temas: anahtarlarını kaydet (OpenSSH'deki accept-new gibi)
skry --accept-new web1 web2

# Yapılandırma dosyasındaki bir grup
skry @production

# Betikler için JSON olarak tek ölçüm
skry @production --once --json | jq '.[] | {name, status, cpu: .metrics.cpu.total_pct}'

# 5432'yi hangi hostlar dinliyor? nginx hangilerinde çalışıyor? Zamanlayıcı her yerde sağlıklı mı?
skry find port 5432 @production
skry find proc nginx @production
skry find service certbot.timer @production

# Güvenlik nabzı, olay anlık görüntüsü, web paneli
skry security @production
skry snapshot @production
skry serve @production
```

skry mevcut SSH kurulumunuzu kullanır: `~/.ssh/config` (`Host` desenleri,
`HostName`, `User`, `Port`, `IdentityFile`, `IdentitiesOnly`, `ProxyJump`,
`Include`, `UserKnownHostsFile`, `HostKeyAlias`, `ConnectTimeout`),
ssh-agent'ınız ve anahtar dosyalarınız. Ajanın tutmadığı parola korumalı
anahtarların kilidi, başlangıçta bir kez sorularak açılır.

Arayüzü hiç sunucu olmadan deneyin: `skry demo`.

## Komut referansı

```
skry [OPTIONS] <TARGETS>...            terminal arayüzünde izle
skry [OPTIONS] <TARGETS>... --once     tek ölçüm; tablo ya da --json
skry find port <PORT> <TARGETS>...     bir TCP portunu hangi hostlar dinliyor
skry find proc <NAME> <TARGETS>...     bir süreci hangi hostlar çalıştırıyor (ad ya da komut satırı)
skry find service <UNIT> <TARGETS>...  bir systemd biriminin hostlardaki durumu
skry security <TARGETS>...             güvenlik nabzı raporu
skry snapshot <TARGETS>... [--out DIR] [--post]
                                       Markdown + JSON olay anlık görüntüsü
skry serve <TARGETS>... [--bind ADDR]  web paneli ve /metrics
skry script [--all]                    uzak betiği yazdır (hiçbir şey çalıştırılmaz)
skry config path|example|check         yapılandırma dosyası yardımcıları
```

Hedefler `~/.ssh/config` içindeki adlar, `user@host:port` (IPv6 için
`[2001:db8::1]:22`) ya da yapılandırma dosyasındaki `@grup` olabilir.

| Genel seçenek | Anlamı |
| --- | --- |
| `--interval <SN>` | Ölçümler arası saniye (varsayılan 2). |
| `--accept-new` | Bilinmeyen host anahtarlarını reddetmek yerine kaydet. Değişmiş anahtarlar her zaman reddedilir. |
| `-c, --config <DOSYA>` | Yapılandırma dosyası (`SKRY_CONFIG` ile de verilebilir). |
| `--json` | Sorgu ve rapor komutları için makinece okunabilir çıktı. |
| `--ssh-config <DOSYA>` | `~/.ssh/config` yerine kullanılacak SSH istemci yapılandırması. |
| `--concurrency <N>` | Aynı anda bağlanılacak en fazla host sayısı (varsayılan 32). |
| `--no-history` | Geçmiş kaydetme. |
| `--no-agent` | ssh-agent kullanma. |
| `--log-file <DOSYA>` | Logları bir dosyaya yaz (terminal arayüzü asla terminale log yazmaz). Daha fazla ayrıntı için `-v`/`-vv`, özel filtre için `SKRY_LOG`. |

**Çıkış kodu:** başarıda `0`, hatalarda (hatalı argüman, yapılandırma sorunu)
`1`, en az bir hosta ulaşılamadığında `2` (`--once`, `find`, `security`,
`snapshot`).

## Terminal arayüzü

| Tuş | İşlem |
| --- | --- |
| `←↑↓→` / `hjkl` | Kutucuklar arasında gezin |
| `Enter` | Host ayrıntıları |
| `Tab` / `Shift-Tab` / `1`–`5` | Panel değiştir: genel bakış, süreçler, ağ ve diskler, servisler ve konteynerler, güvenlik |
| `n` / `p` | Ayrıntı görünümünde sonraki / önceki host |
| `/` | Host adına ya da gruba göre filtrele |
| `f` | Durum filtresi: tümü, sorunlular, erişilemeyenler |
| `o` | Sıralama: durum, ad, CPU, bellek, disk |
| `S` | Güvenlik nabzı |
| `[` / `]` | Zaman çizelgesinde bir dakika geri / ileri |
| `{` / `}` | Zaman çizelgesinde 15 dakika geri / ileri |
| `L` / `End` | Canlıya dön |
| `s` | Görünen hostların (ya da açık hostun) olay anlık görüntüsü |
| `?` | Yardım |
| `q` / `Ctrl-C` | Çık |

## Yapılandırma

Yapılandırma dosyası isteğe bağlıdır. skry önce (her platformda)
`~/.config/skry/config.toml` dosyasına, sonra platformun yapılandırma
dizinine bakar:

| Platform | Yol |
| --- | --- |
| Linux | `~/.config/skry/config.toml` |
| macOS | `~/Library/Application Support/skry/config.toml` |
| Windows | `%APPDATA%\skry\config\config.toml` |

`skry config path` hangi dosyanın kullanıldığını gösterir, `skry config example`
tam açıklamalı bir örnek yazdırır ([docs/config.example.toml](docs/config.example.toml)),
`skry config check` ise sizinkini doğrular.

```toml
interval = 2.0

[thresholds.cpu]
warning = 80.0
critical = 95.0

[groups.web]
hosts = ["web1", "web2", "deploy@web3.example.com:2222"]
allowed_ports = [22, 80, 443]
tls = ["www.example.com:443"]

[groups.production]
hosts = ["@web", "db1"]

[[webhooks]]
kind = "slack"
url = "https://hooks.slack.com/services/..."
events = ["threshold", "unreachable", "security"]
snapshot = true
```

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `interval` | `2.0` | Ölçümler arası saniye. |
| `slow_interval` | `60.0` | Portlar, konteynerler, birimler, girişler ve güncellemelerin toplanma aralığı (saniye). |
| `concurrency` | `32` | Aynı anda bağlanılan host sayısı. |
| `connect_timeout` | `10.0` | TCP + SSH el sıkışması + kimlik doğrulama için saniye (atlama başına). |
| `command_timeout` | `30.0` | Tek bir toplama komutu için saniye. |
| `thresholds.{cpu,memory,disk,load_per_core}` | 80/95, 85/95, 80/90, 1.5/3.0 | `warning` ve `critical` sınırları. Disk, en dolu gerçek dosya sistemidir; yük, 1 dakikalık yükün çekirdek sayısına bölümüdür. |
| `history.enabled` | `true` | Ölçümleri yerelde kaydet. |
| `history.retention_hours` | `24` | Geçmişin tutulacağı süre (saat). |
| `history.detail_interval` | `30` | Tam host kayıtları arası saniye (özetler her ölçümde saklanır). |
| `history.path` | veri dizini | SQLite veritabanı yolu. |
| `baseline.enabled` | `true` | İstatistiksel baseline'lar. |
| `baseline.alpha` | `0.02` | Ölçüm başına EWMA yumuşatma katsayısı. |
| `baseline.z_threshold` | `3.5` | Bir değerin sapma sayılacağı z-skoru eşiği. |
| `baseline.warmup` | `300` | Bir baseline'a güvenilmeden önceki ölçüm sayısı. |
| `baseline.hourly` | `true` | Günün her saati için ayrı baseline. |
| `alerts.cooldown` | `900` | Aynı uyarı için iki bildirim arası saniye. |
| `alerts.notify_resolved` | `true` | Uyarı kalktığında bildir. |
| `alerts.unreachable_after` | `60` | "Erişilemiyor" uyarısından önce geçmesi gereken hata süresi (saniye). |
| `[[webhooks]]` | yok | `kind` (`slack`, `discord`, `generic`), `url`, isteğe bağlı `name`, `events`, `snapshot`. |
| `security.failed_login_threshold` | `50` | 24 saatte bulgu sayılan başarısız SSH girişi sayısı (10 katı kritik). |
| `security.rpm_updates` | `false` | RHEL ailesi hostlarda dnf önbelleğini sorgula (bkz. [uyumluluk](#uyumluluk)). |
| `security.tls` | `[]` | Bu makineden kontrol edilen TLS uç noktaları (başlangıçta, izleme sırasında saatte bir). |
| `security.tls_warning_days` / `tls_critical_days` | `21` / `7` | Sertifika bitiş eşikleri. |
| `web.bind` | `127.0.0.1:9187` | Panel adresi. |
| `web.token` | yok | Erişim token'ı; loopback dışı adresler için zorunlu. `SKRY_WEB_TOKEN` bunu geçersiz kılar. |
| `snapshot.dir` | geçerli dizin | Anlık görüntü dosyalarının yazılacağı yer. |
| `groups.<ad>.hosts` | | Hostlar ya da `@başkagrup`. |
| `groups.<ad>.allowed_ports` | | Dinlemesi beklenen portlar; diğerleri vurgulanır. |
| `groups.<ad>.tls` | | Grup izlenirken kontrol edilen TLS uç noktaları. |

Yazım hataları gözden kaçmasın diye bilinmeyen anahtarlar reddedilir.

## Web paneli ve Prometheus

```sh
skry serve @production                      # http://127.0.0.1:9187/
SKRY_WEB_TOKEN=$(openssl rand -hex 24) skry serve @production --bind 0.0.0.0:9187
```

Panel, terminal arayüzüyle aynı filo ve host görünümlerini gösterir ve
server-sent events ile canlı güncellenir. Salt okunurdur: yalnızca `GET`
yolları vardır.

Loopback dışındaki bir adrese bağlanmak için erişim token'ı gerekir.
İstemciler bunu `Authorization: Bearer <token>` olarak gönderir ya da bir kez
`http://host:9187/?token=<token>` adresini açar (panel token'ı ardından
HttpOnly, SameSite=Strict bir çerezde tutar). Panel makinenizin dışına
açılacaksa önüne TLS sonlandıran bir ters vekil sunucu koyun.

`/metrics`; `skry_up`, `skry_status`, `skry_cpu_usage_percent`,
`skry_memory_usage_percent`, `skry_filesystem_usage_percent`,
`skry_network_receive_bytes_per_second`, `skry_failed_ssh_logins_24h`,
`skry_pending_security_updates` ve `skry_tls_certificate_days_left` gibi
Prometheus gauge'ları sunar:

```yaml
scrape_configs:
  - job_name: skry
    bearer_token: <token>
    static_configs:
      - targets: ["monitor.example.com:9187"]
```

## Geçmiş, baseline'lar ve uyarılar

**Geçmiş.** Her ölçümde host başına küçük bir özet, platformun veri
dizinindeki `history.db` dosyasına yazılır; tam host kayıtları her
`detail_interval` saniyede bir yazılır. Bir saatten eski veriler bir
dakikalık kovalara, altı saatten eskiler beş dakikalık kovalara katlanır;
`retention_hours` süresini aşan her şey silinir. Kovalar içerdikleri en kötü
durumu korur; böylece veri seyreltildiğinde olaylar kaybolmaz.

**Baseline'lar.** skry her host ve metrik için (CPU, bellek, çekirdek başına
yük, disk, ağ giriş/çıkış) ortalama ve varyansın üstel ağırlıklı hareketli
ortalamasını hem genel olarak hem de günün saatine göre tutar. `warmup`
ölçümden sonra z-skoru `z_threshold` değerini aşan bir değer sapma sayılır:
CPU'yu her gece 03:00'te tavan yapan bir yedekleme normaldir, aynı yük öğlen
normal değildir. Yalnızca yukarı yönlü sapmalar bildirilir. Baseline'lar
yeniden başlatmalar arasında korunur.

**Uyarılar.** Koşullar şunlardır: eşik aşımları, baseline sapmaları,
`unreachable_after` süresinden uzun erişilemeyen hostlar, güvenlik bulguları
ve bitiş tarihi yaklaşan TLS sertifikaları. Bir koşul başladığında bir kez
bildirilir; `cooldown` geçene kadar tekrarlanmaz (warning'den critical'a
yükselmedikçe); cooldown içinde gidip gelirse bastırılır; kalktığında
"çözüldü" mesajı gönderilir. Slack ve Discord tek satırlık bir özet alır;
`generic` webhook'lar JSON alır:

```json
{"source":"skry","kind":"firing","host":"db-1","alert":"threshold:cpu","category":"threshold","level":"critical","message":"cpu 97.2% ≥ 95.0% (critical)","timestamp":"2026-09-29T12:00:00Z"}
```

## Güvenlik modeli

- **Uzakta hiçbir şey kurulmaz ya da yazılmaz.** skry normal bir SSH
  oturumu açar ve her ölçümde salt okunur tek bir POSIX `sh` betiği çalıştırır
  (aşağıya bakın). Dosya yükleme, geçici dosya, arkada bırakılan süreç ya da
  yetki yükseltme yoktur. Entegrasyon testleri, izlenen bir hostun dosya
  sisteminin toplama sonrasında değişmediğini doğrular.
- **Uzakta komut çalıştırma özelliği yok.** Uzak kabuğa ulaşan tek girdiler
  rastgele bir bölüm işaretçisi ve `find service` için `[A-Za-z0-9@._:-]`
  kümesine göre doğrulanmış bir systemd birim adıdır. `find port` ve
  `find proc` yerelde filtreler.
- **Host anahtarları.** Anahtarlar; hash'lenmiş girişler, joker karakterler,
  `[host]:port` ve `@revoked` dahil olmak üzere `known_hosts` dosyalarınıza
  göre kontrol edilir. `--accept-new` verilmedikçe bilinmeyen anahtarlar
  reddedilir; verildiğinde kaydedilirler (OpenSSH
  `StrictHostKeyChecking=accept-new` gibi). Değişmiş bir anahtar her zaman
  reddedilir. Atlama hostları da aynı şekilde doğrulanır.
- **Kimlik bilgileri.** skry ssh-agent'ınızı ve anahtar dosyalarınızı kullanır.
  Parolalar bir kez sorulur, anahtarın bellekte çözülmesi için kullanılır ve
  asla saklanmaz ya da loglanmaz. Parolayla kimlik doğrulama desteklenmez;
  dolayısıyla saklanacak bir şey yoktur.
- **Yapılandırmadaki gizli bilgiler.** Webhook URL'leri ve web token'ı loglarda
  ve hata ayıklama çıktısında gizlenir.
- **Web.** Token yapılandırılmadıkça yalnızca loopback; sabit zamanlı token
  karşılaştırması; sıkı Content-Security-Policy; uzaktan gelen tüm metinler
  HTML olarak değil, düz metin olarak gösterilir.
- **En az yetki.** Sıradan, yetkisiz bir hesap yeterlidir. Bazı veriler grup
  üyeliği gerektirir: journal ve auth logları için `adm` ya da
  `systemd-journal`, konteyner istatistikleri için Docker veya Podman soketine
  erişim. Bu yetki yoksa ilgili panel gerekçesiyle birlikte `n/a` gösterir.

## Uzak makinede ne çalışır

`skry script --all` betiğin tam hâlini yazdırır. Özetle, her ölçümde tek komut:

| Bölüm | Kaynak |
| --- | --- |
| CPU | `/proc/stat` (çekirdek başına ve toplam, farklardan) |
| Bellek, swap | `/proc/meminfo` |
| Yük, çalışma süresi | `/proc/loadavg`, `/proc/uptime` |
| Ağ | `/proc/net/dev` |
| Disk I/O, kullanım | `/proc/diskstats`, `df -P` |
| Kimlik | `hostname`, `uname`, `/etc/os-release` |
| Adresler | `ip -o addr` |
| Süreçler | CPU ve bellek farkları için `/proc/[pid]/stat`, `ps -o pid=,user=` (BusyBox uyumlu) |
| Dinleyen portlar | `ss -tlnH`; yoksa `netstat -tln` ve `/proc/net/tcp` |
| Konteynerler | `docker`/`podman` `ps` ve `stats --no-stream` |
| Başarısız birimler | `systemctl --failed --plain --no-legend` |
| Başarısız SSH girişleri | `journalctl` (sshd, 24 saat) ya da `/var/log/auth.log`, `/var/log/secure`, `/var/log/messages` |
| Bekleyen güncellemeler | `apt-get -s dist-upgrade` (simülasyon, önbellekteki listeler), `apk version -l '<'`, isteğe bağlı olarak `dnf -C updateinfo list` |

Yavaş bölümler (portlar, konteynerler, birimler, girişler, güncellemeler,
adresler) her `slow_interval` saniyede bir, geri kalanlar her ölçümde
çalışır. Her komut korumalıdır: eksik bir araç, reddedilen bir izin ya da
takılma (mevcutsa 10 saniyelik `timeout`) hataya değil `n/a`'ya dönüşür.
skry asla `apt update`, `dnf makecache` ya da durumu değiştiren başka bir
komut çalıştırmaz.

## Uyumluluk

| Uzak sistem | Test edildi | Notlar |
| --- | --- | --- |
| Debian 12 | evet | |
| Ubuntu 24.04 | evet | |
| Rocky Linux 9 (RHEL, Alma) | evet | Bekleyen güncellemeler için `security.rpm_updates = true` ve root gerekir: `dnf` yalnızca önbellekten okuyan sorgularda bile kendi log dosyalarına ekleme yapar; bu, skry'nin "hiçbir şey yazma" kuralıyla çeliştiği için isteğe bağlıdır. |
| Alpine 3.20 (BusyBox) | evet | `netstat` ve BusyBox `ps`; güncellemeler için güvenlik meta verisi yoktur. |

Ayrıştırıcılar, bu sistemlerin her birinden alınmış gerçek çıktılara karşı
[`tests/fixtures`](tests/fixtures) içinde test edilir; entegrasyon test
paketi de skry'yi, bir ProxyJump bastion dahil, Docker'daki dört sistemin
SSH sunucularına karşı çalıştırır.

## Geliştirme

```sh
cargo test                                         # birim, ayrıştırıcı ve TUI snapshot testleri
SKRY_DOCKER_TESTS=1 cargo test --test integration_docker   # Docker'da gerçek SSH sunucuları
cargo run -- demo                                  # sentetik bir filoda terminal arayüzü
```

Bkz. [CONTRIBUTING.md](CONTRIBUTING.md).

## rtop'tan ilham

skry, bir sunucuyu yalnızca SSH üzerinden izleme fikrine öncülük eden,
RapidLoop'un [rtop](https://github.com/rapidloop/rtop) projesinden (MIT
lisanslı) ilham almıştır. skry, bu fikri filolara taşıyan bağımsız bir
uygulamadır; bir fork değildir.

## Lisans

Tercihinize bağlı olarak şu lisanslardan biri altında lisanslanmıştır:

- Apache Lisansı, Sürüm 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT lisansı ([LICENSE-MIT](LICENSE-MIT))

Açıkça aksini belirtmediğiniz sürece, Apache-2.0 lisansında tanımlandığı
şekliyle skry'ye dahil edilmek üzere bilerek gönderdiğiniz her katkı, ek
hiçbir koşul olmaksızın yukarıdaki gibi çift lisanslı olacaktır. Bağlayıcı
metinler, İngilizce lisans dosyalarıdır.
