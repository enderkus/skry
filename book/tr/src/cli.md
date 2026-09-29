# Komut satırı referansı

```text
skry [SEÇENEKLER] <HEDEFLER>...                  terminal arayüzünde izle
skry [SEÇENEKLER] <HEDEFLER>... --once [--json]  tek ölçüm, sonra çık
skry find port <PORT> <HEDEFLER>...              bir TCP portunu kim dinliyor
skry find proc <AD> <HEDEFLER>...                bir süreci kim çalıştırıyor
skry find service <BİRİM> <HEDEFLER>...          bir systemd biriminin her yerdeki durumu
skry security <HEDEFLER>...                      güvenlik nabzı raporu
skry snapshot <HEDEFLER>... [--out DIZIN] [--post]
skry serve <HEDEFLER>... [--bind ADRES]          web paneli + /metrics
skry script [--all]                              uzak betiği yazdır
skry config path | example | check               yapılandırma yardımcıları
skry demo                                        hayali bir filoyla arayüz
skry --help | --version
```

## Hedefler

`<HEDEFLER>` geçen her yerde şunları karıştırabilirsiniz: `~/.ssh/config`'teki
isimler, `kullanıcı@sunucu`, `kullanıcı@sunucu:port`, `[ipv6]:port`,
`ssh://kullanıcı@sunucu:port` ve `@grup`. Bkz.
[Çok sayıda sunucu ve gruplar](many-hosts.md).

## Genel seçenekler

Her komutla çalışırlar ve komuttan önce ya da sonra yazılabilirler.

| Seçenek | Anlamı |
| --- | --- |
| `--interval <SN>` | Ölçümler arası saniye (varsayılan 2, en az 0.2) |
| `--accept-new` | Bilinmeyen host anahtarlarını reddetmek yerine kaydet ([Host anahtarları](host-keys.md)) |
| `-c, --config <DOSYA>` | Bu yapılandırma dosyasını kullan (ortam değişkeni `SKRY_CONFIG` de olur) |
| `--json` | `--once`, `find`, `security`, `snapshot` için JSON çıktı |
| `--ssh-config <DOSYA>` | `~/.ssh/config` yerine bu SSH istemci yapılandırmasını kullan |
| `--concurrency <N>` | Aynı anda temas kurulan sunucu sayısı (varsayılan 32) |
| `--no-history` | Geçmiş kaydetme |
| `--no-agent` | SSH ajanını kullanma |
| `--log-file <DOSYA>` | Logları bu dosyaya ekle |
| `-v`, `-vv` | Daha ayrıntılı log (info, debug) |

Ortam değişkenleri: `SKRY_CONFIG` (yapılandırma dosyası), `SKRY_WEB_TOKEN`
(web token'ı), `SKRY_LOG` (log filtresi, ör. `skry=debug`).

## Çıkış kodu

| Kod | Anlamı |
| --- | --- |
| 0 | Başarılı |
| 1 | Hata: yanlış argümanlar, geçersiz yapılandırma, bilinmeyen grup, geçersiz birim adı, … |
| 2 | En az bir sunucuya ulaşılamadı (`--once`, `find`, `security`, `snapshot`) |

Hedef verilmeden çalıştırılan `skry`, yardımı yazdırır ve 1 ile çıkar.

## `skry <hedefler>` — izle

[Terminal arayüzünü](tui.md) açar. `q`'ya basana kadar çalışır.
[Geçmiş](history.md) kaydeder ve [uyarı](alerts.md) gönderir.

## `--once` — tek ölçüm

```sh
skry @production --once
skry @production --once --json
```

Her sunucuya bağlanır, bir saniye arayla **iki** ölçüm alır (hızlar dahil
olsun diye), yazdırır, bağlantıyı keser. `--once` olmadan tek başına `--json`
da aynı şekilde davranır.

Tablo sütunları: HOST, STATUS, CPU, MEM, DISK, LOAD, UPTIME, DETAILS (hata,
eşik aşımları ve bulgular ya da işletim sistemi adı).

JSON: her sunucu için bir nesne içeren bir dizi:

```json
[
  {
    "name": "web1",
    "addr": "deploy@203.0.113.10:22",
    "groups": ["production"],
    "allowed_ports": [22, 80, 443],
    "status": "ok",
    "conn": "connected",
    "error": null,
    "metrics": {
      "ts": "2026-09-29T12:00:01Z",
      "hostname": "web1",
      "os": "Debian GNU/Linux 12 (bookworm)",
      "kernel": "6.1.0-25-amd64",
      "cores": 4,
      "uptime_secs": 1987200.5,
      "cpu": { "total_pct": 23.4, "user_pct": 17.0, "system_pct": 5.1, "iowait_pct": 1.3, "steal_pct": 0.0, "cores": [21.0, 25.2, 24.1, 23.3] },
      "mem": { "total": 16777216000, "used": 6878658560, "used_pct": 41.0, "...": "..." },
      "load": { "one": 1.52, "five": 1.31, "fifteen": 1.12, "running": 2, "total": 312 },
      "disks": [ { "mount": "/", "filesystem": "/dev/sda1", "used_pct": 55.0, "...": "..." } ],
      "ports": { "status": "ok", "value": [ { "port": 22, "addr": "0.0.0.0" } ] },
      "failed_logins": { "status": "unavailable", "value": "logs not readable (needs adm or systemd-journal group)" },
      "...": "..."
    },
    "health": { "level": "ok", "breaches": [] },
    "deviations": [],
    "findings": [],
    "last_update": "2026-09-29T12:00:01Z"
  }
]
```

- `status`: `ok`, `deviation`, `warning`, `critical`, `unreachable`, `pending`.
- `conn`: `connected` ya da `failed`; başarısızsa `error`,
  `{"kind": "...", "message": "...", "since": "..."}` olur; kind şunlardan
  biridir: `auth`, `host_key`, `timeout`, `network`, `protocol`, `config`.
- Kullanılamayabilecek değerler sarmalanır:
  `{"status":"ok","value":…}`, `{"status":"unavailable","value":"neden"}` ya da
  `{"status":"pending"}`.
- Boyutlar bayt, hızlar saniyede bayt, yüzdeler 0–100 arasıdır.

## `find port`

```sh
skry find port 5432 @production
```

```text
HOST    PORT 5432  ADDRESSES
db1     listening  0.0.0.0:5432, [::]:5432
web1    no
old1    unreachable  timeout: timed out while connecting
```

JSON: `[{"host": "db1", "listening": true, "addresses": ["0.0.0.0:5432"], "error": null}]`
(bilinmiyorsa `listening` değeri `null` olur).

## `find proc`

```sh
skry find proc nginx @web
```

Adı **büyük/küçük harf duyarsız** olarak süreç adıyla ve tam komut satırıyla
sizin bilgisayarınızda eşleştirir. Her eşleşmenin PID'sini, kullanıcısını ve
komut satırını ya da `not running` (çalışmıyor) listeler.

JSON: `[{"host": "web1", "matches": [{"pid": 812, "user": "root", "args": "nginx: master process /usr/sbin/nginx"}], "error": null}]`

## `find service`

```sh
skry find service nginx @web          # uzantı yoksa .service eklenir
skry find service certbot.timer @web
```

```text
HOST  STATE           ENABLED   DESCRIPTION
web1  active/running  enabled   A high performance web server
web2  inactive/dead   disabled  A high performance web server
web3  n/a                       n/a (no systemd)
```

İsimde yalnızca `A-Z a-z 0-9 @ . _ : -` kullanılabilir; başka bir şey
bağlanmadan önce reddedilir. Var olmayan bir birim `not installed` (kurulu
değil) gösterir.

## `security`

Bkz. [Güvenlik nabzı](security-pulse.md). JSON:
`{"generated": "...", "hosts": [...], "tls": [...]}`.

## `snapshot`

Bkz. [Olay anlık görüntüleri](snapshots.md). Seçenekler: `--out DIZIN`, `--post`.

## `serve`

Bkz. [Web paneli ve Prometheus](web.md). Seçenek: `--bind ADRES`
(varsayılan `127.0.0.1:9187` ya da `web.bind`).

## `script`

skry'nin her sunucuda çalıştıracağı POSIX sh betiğini hiçbir yere bağlanmadan
yazdırır. `--all` yavaş bölümleri de içerir. Rastgele işaret
`<random-per-run>` olarak gösterilir.

## `config`

| Alt komut | Ne yapar |
| --- | --- |
| `skry config path` | Kullanılan yapılandırma dosyasını yazdırır; yoksa "(not created yet)" ekler |
| `skry config example` | Tam açıklamalı bir örnek yapılandırma yazdırır |
| `skry config check` | Yapılandırmayı yükleyip doğrular; kısa bir özet ya da hatayı yazdırır |

İlk yapılandırmanızı oluşturun:

```sh
mkdir -p ~/.config/skry
skry config example > ~/.config/skry/config.toml
```

(Sonra ihtiyacınız olmayan örnek grupları ve webhook'ları silin.)

## `demo`

Terminal arayüzünü, zamanla değişen yerleşik hayali bir filoyla açar. Hiç SSH
bağlantısı kurulmaz. Tuşları öğrenmek için harikadır.
