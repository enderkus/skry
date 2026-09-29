# Yapılandırma dosyası referansı

**Bir yapılandırma dosyasına ihtiyacınız yok.** skry makul varsayılanlarla
kutudan çıktığı gibi çalışır. Yapılandırma dosyası sunucu grupları, uyarı
webhook'ları, özel eşikler ve web ayarları için kullanışlıdır.

## Dosya nerede

skry aşağıdakilerden ilk geçerli olanı kullanır:

1. `--config DOSYA` ile (ya da `SKRY_CONFIG` değişkeniyle) verilen dosya. O
   dosya yoksa skry hatayla durur.
2. Varsa `~/.config/skry/config.toml` (ya da `$XDG_CONFIG_HOME/skry/config.toml`)
   — **her** platformda.
3. Platformun yapılandırma dizini:

| Platform | Yol |
| --- | --- |
| Linux | `~/.config/skry/config.toml` |
| macOS | `~/Library/Application Support/skry/config.toml` |
| Windows | `%APPDATA%\skry\config\config.toml` |

Varsayılan konumda dosya yoksa sessizce varsayılanlar kullanılır.

```sh
skry config path       # hangi dosya kullanılıyor
skry config example    # tam, açıklamalı bir örnek
skry config check      # dosyanızı doğrula
```

## Biçim

Dosya [TOML](https://toml.io) kullanır: `[bölümler]` altında gruplanmış
`anahtar = değer` satırları. Metinler tırnak içinde, listeler köşeli parantez
içinde; `#` bir yorum başlatır.

> [!IMPORTANT]
> **Bilinmeyen anahtarlar hatadır.** `intervall = 5` ya da
> `[thresholds.cpus]` gibi bir yazım hatası sessizce yok sayılmak yerine,
> satırı gösteren bir mesajla skry'yi durdurur.

## En üst düzey ayarlar

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `interval` | `2.0` | Ölçümler arası saniye. En az 0.2. `--interval` bunu geçersiz kılar. |
| `slow_interval` | `60.0` | Yavaş bölümler (portlar, konteynerler, birimler, girişler, güncellemeler, işletim sistemi, IP'ler) arası saniye. Asla `interval`'dan kısa olmaz. |
| `concurrency` | `32` | Aynı anda temas kurulan sunucu sayısı. `--concurrency` geçersiz kılar. |
| `connect_timeout` | `10.0` | TCP + SSH el sıkışması + giriş için saniye, **atlama başına**. Bir sunucu için `~/.ssh/config`'teki `ConnectTimeout` önceliklidir. |
| `command_timeout` | `30.0` | Bir ölçümün komutuna tanınan saniye. |

## `[thresholds.*]`

Her biri `warning` ve `critical` içeren dört tablo (ikisi de zorunlu,
warning ≤ critical). Bkz. [Sağlık](health.md).

```toml
[thresholds.cpu]            # CPU meşguliyet %, tüm çekirdekler
warning = 80.0
critical = 95.0

[thresholds.memory]         # kullanılan bellek % (önbellekler sayılmaz)
warning = 85.0
critical = 95.0

[thresholds.disk]           # en dolu gerçek dosya sistemi %
warning = 80.0
critical = 90.0

[thresholds.load_per_core]  # 1 dakikalık yük ÷ çekirdek
warning = 1.5
critical = 3.0
```

## `[history]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `enabled` | `true` | Geçmiş kaydet (yalnızca terminal arayüzü ve `serve`). `--no-history` geçersiz kılar. |
| `retention_hours` | `24` | Bundan eski verileri sil. |
| `detail_interval` | `30` | Tam sunucu kayıtları arası saniye. |
| `path` | platform veri dizini | Veritabanı dosyası, ör. `"/var/lib/skry/history.db"`. |

Bkz. [Geçmiş ve zamanda yolculuk](history.md).

## `[baseline]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `enabled` | `true` | Baseline öğren ve sapmaları işaretle. |
| `alpha` | `0.02` | Ölçüm başına yumuşatma katsayısı, 0 ile 1 arasında. |
| `z_threshold` | `3.5` | Bir değerin sapma sayıldığı z-skoru eşiği. |
| `warmup` | `300` | Bir baseline kullanılmadan önce gereken ölçüm sayısı. |
| `hourly` | `true` | Günün her saati için ayrı bir baseline tut. |

Bkz. [Baseline'lar](baselines.md).

## `[alerts]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `cooldown` | `900` | Aynı uyarı için iki bildirim arası saniye. |
| `notify_resolved` | `true` | Uyarı kalktığında mesaj gönder. |
| `unreachable_after` | `60` | "unreachable" tetiklenmeden önce sunucunun hata vermesi gereken saniye. |

## `[[webhooks]]`

Her hedef için bloğu tekrarlayın (çift köşeli paranteze dikkat).

| Anahtar | Zorunlu | Anlamı |
| --- | --- | --- |
| `kind` | evet | `"slack"`, `"discord"` ya da `"generic"` |
| `url` | evet | Webhook adresi (`http://` ya da `https://`) |
| `name` | hayır | Loglarda kullanılan bir etiket |
| `events` | hayır | İletilecek kategoriler: `"threshold"`, `"baseline"`, `"unreachable"`, `"security"` (verilmezse hepsi) |
| `snapshot` | hayır | Olay anlık görüntülerini de almak için `true` |

Bkz. [Uyarılar ve webhook'lar](alerts.md).

## `[security]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `failed_login_threshold` | `50` | 24 saatte bulgu oluşturan başarısız SSH girişi sayısı (10 katı = critical). `0` bulguyu kapatır. |
| `rpm_updates` | `false` | RHEL ailesi sunucularda dnf önbelleğini sorgula (root gerekir, dnf kendi loglarını yazar). |
| `tls` | `[]` | Sizin bilgisayarınızdan kontrol edilen TLS uç noktaları, ör. `["www.example.com:443"]`. |
| `tls_warning_days` | `21` | Sertifika bu kadar gün içinde bitiyorsa uyar. |
| `tls_critical_days` | `7` | Bu kadar gün içinde bitiyorsa (ya da geçersizse) kritik. |

Bkz. [Güvenlik nabzı](security-pulse.md).

## `[web]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `bind` | `"127.0.0.1:9187"` | `skry serve` için dinleme adresi. `--bind` geçersiz kılar. |
| `token` | yok | Erişim token'ı. Loopback olmayan adresler için zorunlu. `SKRY_WEB_TOKEN` geçersiz kılar. |

Bkz. [Web paneli](web.md).

## `[snapshot]`

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `dir` | geçerli dizin | Anlık görüntü dosyalarının yazılacağı yer. `~` genişletilir. |

## `[groups.<ad>]`

| Anahtar | Anlamı |
| --- | --- |
| `hosts` | Hedef listesi: SSH yapılandırmasındaki isimler, `kullanıcı@sunucu:port` ya da `"@başkagrup"` |
| `allowed_ports` | Bu grubun dinleyebileceği TCP portları; diğerleri işaretlenir. Boş = politika yok. |
| `tls` | Bu grup izlenirken kontrol edilen TLS uç noktaları |

```toml
[groups.web]
hosts = ["web1", "web2", "deploy@web3.example.com:2222"]
allowed_ports = [22, 80, 443]
tls = ["www.example.com:443"]

[groups.production]
hosts = ["@web", "db1"]
```

`skry @web`, `skry @production` ile kullanın.

## Başka yerde duran ayarlar

- **Nasıl bağlanılacağı** (adresler, kullanıcılar, portlar, anahtarlar, atlama
  sunucuları) skry yapılandırmasında değildir. `~/.ssh/config`'ten gelir;
  böylece `ssh` ile skry her zaman aynı fikirdedir.
- **Gizli bilgiler**: dosyadaki tek gizli bilgiler webhook adresleri ve web
  token'ıdır. Dosyayı yalnızca sizin okuyabileceğiniz şekilde tutun
  (`chmod 600`). skry bunları loglarda asla yazdırmaz.
