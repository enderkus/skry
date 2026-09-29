# Example configurations

Copy the one closest to your situation and adapt it. Each is a complete file.

## 1. "I just want groups"

```toml
[groups.home]
hosts = ["nas", "pi-hole", "media"]

[groups.work]
hosts = ["web1", "web2", "db1"]
```

## 2. A small team with Slack alerts

```toml
interval = 5.0              # a bit calmer than the default 2 s

[thresholds.disk]
warning = 85.0
critical = 95.0

[alerts]
cooldown = 1800             # at most one message per alert every 30 min

[[webhooks]]
name = "team"
kind = "slack"
url = "https://hooks.slack.com/services/<your-webhook-path>"
events = ["threshold", "unreachable", "security"]
snapshot = true

[groups.production]
hosts = ["web1", "web2", "db1", "cache1"]
allowed_ports = [22, 80, 443]

[security]
tls = ["www.example.com:443", "api.example.com:443"]
```

## 3. A monitoring box running `skry serve` for Prometheus

```toml
interval = 10.0
concurrency = 64

[history]
retention_hours = 48
path = "/var/lib/skry/history.db"

[web]
bind = "0.0.0.0:9187"
# token comes from SKRY_WEB_TOKEN in the systemd unit

[baseline]
z_threshold = 4.0

[[webhooks]]
name = "on-call"
kind = "generic"
url = "https://alerts.example.com/skry"
events = ["unreachable", "threshold"]

[groups.all]
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

## 4. Servers behind a bastion

skry config (`~/.config/skry/config.toml`):

```toml
[groups.private]
hosts = ["app1", "app2", "db1"]
```

SSH config (`~/.ssh/config`) does the routing:

```text
Host bastion
    HostName bastion.example.com
    User ops
    IdentityFile ~/.ssh/ops_ed25519

Host app1 app2 db1
    ProxyJump bastion
    User skry
    IdentityFile ~/.ssh/skry_monitoring
    IdentitiesOnly yes

Host app1
    HostName 10.0.1.11
Host app2
    HostName 10.0.1.12
Host db1
    HostName 10.0.2.21
```

## 5. RHEL/Rocky fleet with update checks

```toml
[security]
rpm_updates = true          # skry logs in as root on these hosts (see Permissions)
failed_login_threshold = 100

[groups.rhel]
hosts = ["root@rocky1", "root@rocky2"]
```

## 6. Quiet mode: no history, no baselines

```toml
[history]
enabled = false

[baseline]
enabled = false
```
