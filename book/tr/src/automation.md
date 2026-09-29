# Betikler ve otomasyon

## JSON + jq tarifleri

```sh
# Sağlıksız sunucuların adları
skry @production --once --json | jq -r '.[] | select(.status != "ok") | .name'

# CSV olarak CPU ve bellek
skry @production --once --json \
  | jq -r '.[] | [.name, .metrics.cpu.total_pct, .metrics.mem.used_pct] | @csv'

# Bekleyen güvenlik güncellemesi olan sunucular
skry security @production --json \
  | jq -r '.hosts[] | select(.updates.status == "ok" and .updates.value.security > 0) | .host'

# redis hangi sunucularda çalışıyor?
skry find proc redis-server @production --json | jq -r '.[] | select(.matches | length > 0) | .host'
```

## Çıkış kodunu kullanmak

```sh
# Herhangi bir sunucuya ulaşılamıyorsa bir CI işini ya da cron kontrolünü düşür
if ! skry @production --once > /tmp/filo.txt; then
    echo "bazı sunuculara ulaşılamıyor"; cat /tmp/filo.txt
fi
```

Çıkış kodu 2 "en az bir sunucuya ulaşılamadı", 1 ise kullanım ya da
yapılandırma hatası demektir. Bkz.
[Komut satırı referansı](cli.md#çıkış-kodu).

## Etkileşimsiz çalıştırmalar

skry cron'dan, CI'dan ya da systemd'den çalıştığında:

- Anahtar parolası sormak için bir terminal yoktur. Bir SSH ajanı kullanın ya da
  bir izleme hesabıyla sınırlı, parolasız ayrı bir anahtar kullanın.
- Host anahtarları `known_hosts`'ta zaten bulunmalıdır (ya da bir kez
  `--accept-new` ile çalıştırın).
- Log tutmak için `--log-file` ve `-v` kullanın.

## `skry serve`'ü servis olarak çalıştırmak (Linux, systemd)

`/etc/systemd/system/skry.service`:

```ini
[Unit]
Description=skry fleet monitoring
After=network-online.target
Wants=network-online.target

[Service]
User=monitor
Environment=SKRY_WEB_TOKEN=degistirin
ExecStart=/usr/local/bin/skry serve @production --bind 0.0.0.0:9187 --log-file /home/monitor/skry.log
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now skry
```

`monitor` kullanıcısının kendi `~/.ssh/config`, anahtar ve `known_hosts`
dosyalarına (gruplar ve webhook'lar için de `~/.config/skry/config.toml`
dosyasına) ihtiyacı vardır. Bu size kesintisiz uyarı, geçmiş ve bir Prometheus
uç noktası sağlar.

## Sohbete günlük güvenlik raporu

```sh
# crontab -e
0 8 * * 1-5  /usr/local/bin/skry snapshot @production --post --out /var/tmp/skry >/dev/null 2>&1
```

`snapshot = true` olan bir webhook ile ekip her sabah bir özet alır.

## Dağıtım öncesi kontrol

```sh
skry find port 8080 @web --json \
  | jq -e 'all(.[]; .listening == false)' > /dev/null \
  || { echo "8080 portu bir yerde zaten kullanımda"; exit 1; }
```
