# Web dashboard and Prometheus

`skry serve` runs the same monitoring as the terminal UI, but shows it in a
browser and exposes a Prometheus endpoint. It is handy on a shared screen, for
colleagues who do not use a terminal, and for feeding Grafana.

```sh
skry serve @production
```

```text
skry dashboard on http://127.0.0.1:9187/ and metrics on http://127.0.0.1:9187/metrics
```

Open the address in a browser. Stop with `Ctrl-C`.

## What the dashboard shows

- **Fleet**: one tile per host, coloured by status, with CPU, memory and disk
  bars, load, network rates, deviation and security flags. Filter by name or
  group, filter by status, sort — like the terminal UI.
- **Host details**: click a tile. A side panel shows everything from the
  terminal's host view: CPU split and per-core bars, memory, filesystems, disk
  I/O, interfaces, processes, containers, failed units, ports, failed logins,
  updates. `Esc` closes it.
- **Security**: the security pulse table and TLS certificates.
- The top right shows `LIVE` while connected. Updates arrive by themselves
  (server-sent events), at most twice per second; if the connection drops,
  the browser reconnects automatically.
- Tile colours follow **your** configured thresholds.

The dashboard is **read-only**: there are no buttons that change anything,
and the server accepts only `GET` requests.

## Access from other machines: the token rule

By default the dashboard listens on `127.0.0.1:9187`, reachable only from the
same computer. To make it reachable from the network you must choose a
**different address and set an access token**; without a token skry refuses
to start:

```text
error: refusing to bind to non-loopback address 0.0.0.0:9187 without an access token; …
```

Set a token, either in the config:

```toml
[web]
bind = "0.0.0.0:9187"
token = "a-long-random-string"
```

or, preferably (it keeps the secret out of files), as an environment variable
that overrides the config:

```sh
export SKRY_WEB_TOKEN="$(openssl rand -hex 24)"
skry serve @production --bind 0.0.0.0:9187
```

Clients prove they know the token in one of three ways:

| Way | Used by |
| --- | --- |
| `Authorization: Bearer <token>` header | Prometheus, `curl`, scripts |
| `?token=<token>` in the URL, once | Browsers: open `http://host:9187/?token=…`; the dashboard then stores the token in a cookie (HttpOnly, SameSite=Strict) and removes it from the address bar |
| `skry_token` cookie | Browsers, after the step above |

If a token is configured it is required on loopback too.

> [!WARNING]
> skry serves plain HTTP. When the dashboard leaves your machine, put it
> behind a reverse proxy that terminates TLS (nginx, Caddy, Traefik), or reach
> it through an SSH tunnel: `ssh -L 9187:127.0.0.1:9187 monitor-host`.

## HTTP endpoints

| Path | Returns |
| --- | --- |
| `/` | The dashboard |
| `/api/fleet` | The complete fleet state as JSON: `hosts` (each host in the same shape as `--once --json`), `tls`, `started`, `generation` |
| `/api/host/<name>` | One host as JSON (404 if unknown) |
| `/api/events` | Server-sent events stream; each `fleet` event carries the full fleet JSON |
| `/api/thresholds` | The configured thresholds |
| `/metrics` | Prometheus text format |
| `/healthz` | `ok` |

## Prometheus

Add a scrape job:

```yaml
scrape_configs:
  - job_name: skry
    scrape_interval: 15s
    bearer_token: <token>          # only if a token is set
    static_configs:
      - targets: ["monitor.example.com:9187"]
```

Every series has a `host` label with the name you gave skry.

| Metric | Labels | Meaning |
| --- | --- | --- |
| `skry_up` | host | 1 if skry can read the host, 0 if not |
| `skry_status` | host | 0 ok, 1 deviation, 2 warning, 3 critical, 4 unreachable |
| `skry_cpu_usage_percent` | host | CPU busy % |
| `skry_cpu_core_usage_percent` | host, core | per core |
| `skry_cpu_cores` | host | number of cores |
| `skry_memory_used_bytes`, `skry_memory_total_bytes`, `skry_memory_usage_percent` | host | memory |
| `skry_swap_used_bytes`, `skry_swap_total_bytes` | host | swap |
| `skry_load1`, `skry_load5`, `skry_load15` | host | load averages |
| `skry_uptime_seconds` | host | uptime |
| `skry_network_receive_bytes_per_second`, `skry_network_transmit_bytes_per_second` | host, interface | rates |
| `skry_network_receive_bytes_total`, `skry_network_transmit_bytes_total` | host, interface | counters since boot |
| `skry_filesystem_used_bytes`, `skry_filesystem_size_bytes`, `skry_filesystem_usage_percent` | host, mount, device | filesystems |
| `skry_disk_read_bytes_per_second`, `skry_disk_write_bytes_per_second`, `skry_disk_utilization_percent` | host, device | disk I/O |
| `skry_processes` | host | number of processes |
| `skry_failed_units` | host | failed systemd units |
| `skry_failed_ssh_logins_24h` | host | failed SSH logins |
| `skry_pending_updates`, `skry_pending_security_updates` | host | pending updates |
| `skry_listening_ports` | host | listening TCP sockets |
| `skry_security_findings` | host | security findings |
| `skry_containers` | host, state | containers per state |
| `skry_baseline_deviations` | host | metrics currently deviating |
| `skry_tls_certificate_days_left`, `skry_tls_certificate_valid` | endpoint | TLS checks |

Metrics that are `n/a` on a host (for example no permission to read logs)
are simply absent for that host.

**Example queries**

```text
skry_up == 0                                        # hosts skry cannot reach
max by (host) (skry_filesystem_usage_percent) > 85  # nearly full disks
skry_pending_security_updates > 0                   # hosts needing patches
skry_tls_certificate_days_left < 14                 # certificates to renew
topk(5, skry_cpu_usage_percent)                     # busiest hosts
```

The values are skry's latest sample at scrape time. Prometheus gives you long
retention and Grafana graphs; skry stays agentless.
