# Web paneli ve Prometheus

`skry serve`, terminal arayüzüyle aynı izlemeyi yapar ama sonucu bir
tarayıcıda gösterir ve bir Prometheus uç noktası sunar. Ortak bir ekranda,
terminal kullanmayan çalışma arkadaşları için ve Grafana'yı beslemek için
kullanışlıdır.

```sh
skry serve @production
```

```text
skry dashboard on http://127.0.0.1:9187/ and metrics on http://127.0.0.1:9187/metrics
```

Adresi bir tarayıcıda açın. `Ctrl-C` ile durdurun.

## Panel neyi gösterir

- **Filo**: her sunucu için durumuna göre renklenen, CPU, bellek ve disk
  çubukları, yük, ağ hızları, sapma ve güvenlik işaretleri olan bir kutucuk.
  İsme ya da gruba göre filtreleme, duruma göre filtreleme, sıralama — terminal
  arayüzündeki gibi.
- **Sunucu ayrıntıları**: bir kutucuğa tıklayın. Yan panel, terminalin sunucu
  görünümündeki her şeyi gösterir: CPU dağılımı ve çekirdek çubukları, bellek,
  dosya sistemleri, disk I/O, arayüzler, süreçler, konteynerler, çökmüş
  birimler, portlar, başarısız girişler, güncellemeler. `Esc` kapatır.
- **Security**: güvenlik nabzı tablosu ve TLS sertifikaları.
- Sağ üst, bağlıyken `LIVE` gösterir. Güncellemeler kendiliğinden gelir
  (server-sent events), saniyede en fazla iki kez; bağlantı koparsa tarayıcı
  otomatik olarak yeniden bağlanır.
- Kutucuk renkleri **sizin** yapılandırdığınız eşikleri izler.

Panel **salt okunurdur**: hiçbir şeyi değiştiren bir düğme yoktur ve sunucu
yalnızca `GET` isteklerini kabul eder.

## Başka makinelerden erişim: token kuralı

Panel varsayılan olarak `127.0.0.1:9187` üzerinde dinler; yalnızca aynı
bilgisayardan erişilebilir. Ağdan erişilebilir yapmak için **farklı bir adres
seçmeli ve bir erişim token'ı belirlemelisiniz**; token yoksa skry başlamayı
reddeder:

```text
error: refusing to bind to non-loopback address 0.0.0.0:9187 without an access token; …
```

Token'ı ya yapılandırmada belirleyin:

```toml
[web]
bind = "0.0.0.0:9187"
token = "uzun-rastgele-bir-metin"
```

ya da tercihen (gizli bilgiyi dosyalardan uzak tutar) yapılandırmayı
geçersiz kılan bir ortam değişkeniyle:

```sh
export SKRY_WEB_TOKEN="$(openssl rand -hex 24)"
skry serve @production --bind 0.0.0.0:9187
```

İstemciler token'ı bildiklerini üç yoldan biriyle kanıtlar:

| Yol | Kim kullanır |
| --- | --- |
| `Authorization: Bearer <token>` başlığı | Prometheus, `curl`, betikler |
| Adreste bir kez `?token=<token>` | Tarayıcılar: `http://sunucu:9187/?token=…` açın; panel token'ı bir çereze (HttpOnly, SameSite=Strict) kaydeder ve adres çubuğundan siler |
| `skry_token` çerezi | Yukarıdaki adımdan sonra tarayıcılar |

Bir token yapılandırılmışsa loopback'te de istenir.

> [!WARNING]
> skry düz HTTP sunar. Panel makinenizin dışına çıkacaksa onu TLS sonlandıran
> bir ters vekil sunucunun (nginx, Caddy, Traefik) arkasına koyun ya da bir SSH
> tüneliyle erişin: `ssh -L 9187:127.0.0.1:9187 izleme-sunucusu`.

## HTTP uç noktaları

| Yol | Döndürdüğü |
| --- | --- |
| `/` | Panel |
| `/api/fleet` | Tüm filo durumu JSON olarak: `hosts` (her sunucu `--once --json` ile aynı biçimde), `tls`, `started`, `generation` |
| `/api/host/<isim>` | Tek bir sunucu JSON olarak (bilinmiyorsa 404) |
| `/api/events` | Server-sent events akışı; her `fleet` olayı tüm filo JSON'unu taşır |
| `/api/thresholds` | Yapılandırılmış eşikler |
| `/metrics` | Prometheus metin biçimi |
| `/healthz` | `ok` |

## Prometheus

Bir scrape işi ekleyin:

```yaml
scrape_configs:
  - job_name: skry
    scrape_interval: 15s
    bearer_token: <token>          # yalnızca token belirlendiyse
    static_configs:
      - targets: ["izleme.example.com:9187"]
```

Her serinin, skry'ye verdiğiniz isimle bir `host` etiketi vardır.

| Metrik | Etiketler | Anlamı |
| --- | --- | --- |
| `skry_up` | host | skry sunucuyu okuyabiliyorsa 1, okuyamıyorsa 0 |
| `skry_status` | host | 0 ok, 1 deviation, 2 warning, 3 critical, 4 unreachable |
| `skry_cpu_usage_percent` | host | CPU meşguliyet % |
| `skry_cpu_core_usage_percent` | host, core | çekirdek başına |
| `skry_cpu_cores` | host | çekirdek sayısı |
| `skry_memory_used_bytes`, `skry_memory_total_bytes`, `skry_memory_usage_percent` | host | bellek |
| `skry_swap_used_bytes`, `skry_swap_total_bytes` | host | swap |
| `skry_load1`, `skry_load5`, `skry_load15` | host | yük ortalamaları |
| `skry_uptime_seconds` | host | çalışma süresi |
| `skry_network_receive_bytes_per_second`, `skry_network_transmit_bytes_per_second` | host, interface | hızlar |
| `skry_network_receive_bytes_total`, `skry_network_transmit_bytes_total` | host, interface | açılıştan beri sayaçlar |
| `skry_filesystem_used_bytes`, `skry_filesystem_size_bytes`, `skry_filesystem_usage_percent` | host, mount, device | dosya sistemleri |
| `skry_disk_read_bytes_per_second`, `skry_disk_write_bytes_per_second`, `skry_disk_utilization_percent` | host, device | disk I/O |
| `skry_processes` | host | süreç sayısı |
| `skry_failed_units` | host | çökmüş systemd birimleri |
| `skry_failed_ssh_logins_24h` | host | başarısız SSH girişleri |
| `skry_pending_updates`, `skry_pending_security_updates` | host | bekleyen güncellemeler |
| `skry_listening_ports` | host | dinleyen TCP soketleri |
| `skry_security_findings` | host | güvenlik bulguları |
| `skry_containers` | host, state | duruma göre konteynerler |
| `skry_baseline_deviations` | host | şu an sapan metrikler |
| `skry_tls_certificate_days_left`, `skry_tls_certificate_valid` | endpoint | TLS kontrolleri |

Bir sunucuda `n/a` olan metrikler (örneğin logları okuma izni yok) o sunucu
için hiç yer almaz.

**Örnek sorgular**

```text
skry_up == 0                                        # skry'nin ulaşamadığı sunucular
max by (host) (skry_filesystem_usage_percent) > 85  # dolmak üzere olan diskler
skry_pending_security_updates > 0                   # yama bekleyen sunucular
skry_tls_certificate_days_left < 14                 # yenilenecek sertifikalar
topk(5, skry_cpu_usage_percent)                     # en meşgul sunucular
```

Değerler, scrape anındaki en son skry ölçümüdür. Uzun saklama ve Grafana
grafikleri Prometheus'tan; skry ise ajansız kalır.
