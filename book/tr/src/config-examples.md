# Örnek yapılandırmalar

Durumunuza en yakın olanı kopyalayıp uyarlayın. Her biri eksiksiz bir dosyadır.

## 1. "Sadece grup istiyorum"

```toml
[groups.ev]
hosts = ["nas", "pi-hole", "medya"]

[groups.is]
hosts = ["web1", "web2", "db1"]
```

## 2. Slack uyarılı küçük bir ekip

```toml
interval = 5.0              # varsayılan 2 sn'den biraz daha sakin

[thresholds.disk]
warning = 85.0
critical = 95.0

[alerts]
cooldown = 1800             # uyarı başına en fazla 30 dakikada bir mesaj

[[webhooks]]
name = "ekip"
kind = "slack"
url = "https://hooks.slack.com/services/<webhook-yolunuz>"
events = ["threshold", "unreachable", "security"]
snapshot = true

[groups.production]
hosts = ["web1", "web2", "db1", "cache1"]
allowed_ports = [22, 80, 443]

[security]
tls = ["www.example.com:443", "api.example.com:443"]
```

## 3. Prometheus için `skry serve` çalıştıran bir izleme sunucusu

```toml
interval = 10.0
concurrency = 64

[history]
retention_hours = 48
path = "/var/lib/skry/history.db"

[web]
bind = "0.0.0.0:9187"
# token, systemd birimindeki SKRY_WEB_TOKEN'dan gelir

[baseline]
z_threshold = 4.0

[[webhooks]]
name = "nobetci"
kind = "generic"
url = "https://alerts.example.com/skry"
events = ["unreachable", "threshold"]

[groups.hepsi]
hosts = ["@web", "@db", "@workers"]

[groups.web]
hosts = ["web01", "web02", "web03", "web04"]
allowed_ports = [22, 80, 443, 9100]

[groups.db]
hosts = ["db01", "db02"]
allowed_ports = [22, 5432]

[groups.workers]
hosts = ["worker01", "worker02", "worker03"]
allowed_ports = [22]
```

## 4. Bastion arkasındaki sunucular

skry yapılandırması (`~/.config/skry/config.toml`):

```toml
[groups.ozel]
hosts = ["app1", "app2", "db1"]
```

Yönlendirmeyi SSH yapılandırması (`~/.ssh/config`) yapar:

```text
Host bastion
    HostName bastion.example.com
    User ops
    IdentityFile ~/.ssh/ops_ed25519

Host app1 app2 db1
    ProxyJump bastion
    User skry
    IdentityFile ~/.ssh/skry_izleme
    IdentitiesOnly yes

Host app1
    HostName 10.0.1.11
Host app2
    HostName 10.0.1.12
Host db1
    HostName 10.0.2.21
```

## 5. Güncelleme kontrollü RHEL/Rocky filosu

```toml
[security]
rpm_updates = true          # skry bu sunuculara root olarak girer (bkz. İzinler)
failed_login_threshold = 100

[groups.rhel]
hosts = ["root@rocky1", "root@rocky2"]
```

## 6. Sessiz mod: geçmiş yok, baseline yok

```toml
[history]
enabled = false

[baseline]
enabled = false
```
