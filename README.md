# skry

**English** · [Türkçe](README.tr.md)

**See every server. Install nothing.**

[![CI](https://github.com/enderkus/skry/actions/workflows/ci.yml/badge.svg)](https://github.com/enderkus/skry/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

skry monitors one or many Linux servers **over plain SSH**, with **nothing
installed or written on the remote machines**. It reads `/proc` and a few
standard commands, and shows the results in a fast terminal UI, an optional
web dashboard, a Prometheus endpoint, and scriptable JSON.

![skry demo](docs/demo.gif)

- **Agentless.** No daemon, no copied binary, no temp files, no sudo.
- **Read-only.** skry never changes a remote host and offers no way to run
  arbitrary commands across the fleet.
- **Secure by default.** `known_hosts` is honoured and unknown host keys are
  refused unless you pass `--accept-new`. Passwords are never stored.
- **Cheap.** One SSH session per host, one batched POSIX `sh` command per
  tick.
- **Portable.** Works on Debian, Ubuntu, RHEL/Rocky/Alma and Alpine
  (BusyBox). Anything missing or not permitted shows up as `n/a`.
- **One binary** for Linux, macOS and Windows.

## Contents

- [What you get](#what-you-get)
- [Installation](#installation)
- [Quickstart](#quickstart)
- [Command reference](#command-reference)
- [Terminal UI](#terminal-ui)
- [Configuration](#configuration)
- [Web dashboard and Prometheus](#web-dashboard-and-prometheus)
- [History, baselines and alerts](#history-baselines-and-alerts)
- [Security model](#security-model)
- [What runs on the remote host](#what-runs-on-the-remote-host)
- [Compatibility](#compatibility)
- [Inspired by rtop](#inspired-by-rtop)
- [License](#license)

## What you get

| | |
| --- | --- |
| **Fleet view** | A grid of host tiles coloured by health (ok, warning, critical, unreachable, baseline deviation) with CPU, memory, disk and load at a glance. Sort and filter by name, group or status. Authentication failures, timeouts and host key problems are shown on the tile; one failing host never blocks the others. |
| **Host view** | CPU (total and per core), memory and swap, load, uptime, network rates per interface, disk usage and I/O per device, top processes, containers (Docker/Podman), failed systemd units, OS and kernel. |
| **Time travel** | Samples are kept in a local SQLite database (24 hours by default, downsampled automatically). Scrub back along the timeline to see the fleet or a host as it was. |
| **Security pulse** | Failed SSH logins in the last 24 hours, pending security updates (from cached package metadata only), listening ports outside a per-group allowlist, and TLS certificate expiry checked from your machine. |
| **Baselines** | An EWMA of mean and variance per host and metric, optionally per hour of day. Values that are unusual *for that host* are flagged, alongside static thresholds. |
| **Alerts** | Slack, Discord and generic JSON webhooks for threshold breaches, baseline deviations, unreachable hosts and security findings, with deduplication and cooldown. |
| **Incident snapshots** | Press `s` (or run `skry snapshot`) to capture the full state of the selected hosts as Markdown and JSON, optionally posted to a webhook. |
| **Web + Prometheus** | `skry serve` offers a read-only dashboard with live updates and a `/metrics` endpoint. |

## Installation

skry runs on your workstation (or a jump box). The monitored hosts only need
an SSH server and a POSIX shell.

### Quick install

**Linux and macOS**

```sh
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | sh
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

The installer detects your platform, downloads the latest release, verifies
its SHA-256 checksum and installs the binary:

| Platform | Installed to |
| --- | --- |
| Linux, macOS | `/usr/local/bin` if writable, otherwise `~/.local/bin` |
| Windows | `%LOCALAPPDATA%\Programs\skry`, added to your user `PATH` |

Two optional environment variables change the defaults:

```sh
# A specific version, into a directory of your choice
curl -fsSL https://raw.githubusercontent.com/enderkus/skry/main/install.sh | SKRY_VERSION=v0.1.0 SKRY_INSTALL_DIR=/opt/bin sh
```

```powershell
$env:SKRY_VERSION = 'v0.1.0'; irm https://raw.githubusercontent.com/enderkus/skry/main/install.ps1 | iex
```

Prefer to read a script before running it? Download
[`install.sh`](install.sh) or [`install.ps1`](install.ps1), look it over,
then run it locally.

### Manual download (v0.1.0)

| Platform | Archive | Checksum |
| --- | --- | --- |
| Linux x86_64 (static) | [skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256) |
| Linux aarch64 (static) | [skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-unknown-linux-musl.tar.gz.sha256) |
| macOS Apple Silicon | [skry-v0.1.0-aarch64-apple-darwin.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256) |
| macOS Intel | [skry-v0.1.0-x86_64-apple-darwin.tar.gz](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-apple-darwin.tar.gz) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-apple-darwin.tar.gz.sha256) |
| Windows x86_64 | [skry-v0.1.0-x86_64-pc-windows-msvc.zip](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-pc-windows-msvc.zip) | [sha256](https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-pc-windows-msvc.zip.sha256) |

Newer releases are listed on the [releases page](https://github.com/enderkus/skry/releases).

**Linux** (use `aarch64` instead of `x86_64` on ARM)

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
sha256sum -c skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
tar xzf skry-v0.1.0-x86_64-unknown-linux-musl.tar.gz
sudo install skry-v0.1.0-x86_64-unknown-linux-musl/skry /usr/local/bin/
```

**macOS** (use `x86_64` instead of `aarch64` on Intel)

```sh
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz
curl -LO https://github.com/enderkus/skry/releases/download/v0.1.0/skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
shasum -a 256 -c skry-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar xzf skry-v0.1.0-aarch64-apple-darwin.tar.gz
sudo install skry-v0.1.0-aarch64-apple-darwin/skry /usr/local/bin/
```

Archives downloaded with a browser are quarantined by Gatekeeper; clear the
flag once with `xattr -d com.apple.quarantine /usr/local/bin/skry`.

**Windows**

Download the zip, extract it and move `skry.exe` to a folder on your `PATH`.
skry talks to the Windows OpenSSH agent (`ssh-agent` service) or Pageant, and
reads `%USERPROFILE%\.ssh\config` and `known_hosts`. Use Windows Terminal for
the best TUI rendering.

### From source

With a recent stable Rust toolchain (1.85 or newer):

```sh
cargo install --locked --git https://github.com/enderkus/skry
```

### Uninstall

Delete the binary (`/usr/local/bin/skry`, `~/.local/bin/skry`, or
`%LOCALAPPDATA%\Programs\skry` on Windows, where you can also remove the
entry from your user `PATH`). skry keeps nothing else except, if you used
them, its config file and history database (`skry config path` shows the
config location; history lives in the platform data directory, e.g.
`~/.local/share/skry` on Linux or `~/Library/Application Support/skry` on
macOS).

## Quickstart

```sh
# A host from ~/.ssh/config
skry web1

# Several hosts, mixing aliases and explicit user@host:port
skry web1 web2 deploy@10.0.0.7:2222

# First contact with new hosts: record their keys (like accept-new in OpenSSH)
skry --accept-new web1 web2

# A group from the config file
skry @production

# One sample as JSON, for scripts
skry @production --once --json | jq '.[] | {name, status, cpu: .metrics.cpu.total_pct}'

# Which hosts listen on 5432? Which run nginx? Is the timer healthy everywhere?
skry find port 5432 @production
skry find proc nginx @production
skry find service certbot.timer @production

# Security pulse, incident snapshot, web dashboard
skry security @production
skry snapshot @production
skry serve @production
```

skry uses your existing SSH setup: `~/.ssh/config` (`Host` patterns,
`HostName`, `User`, `Port`, `IdentityFile`, `IdentitiesOnly`, `ProxyJump`,
`Include`, `UserKnownHostsFile`, `HostKeyAlias`, `ConnectTimeout`), your
ssh-agent, and your key files. Passphrase-protected keys that the agent does
not hold are unlocked once at start-up with a prompt.

Try the interface without any servers: `skry demo`.

## Command reference

```
skry [OPTIONS] <TARGETS>...            monitor in the TUI
skry [OPTIONS] <TARGETS>... --once     one sample, table or --json
skry find port <PORT> <TARGETS>...     which hosts listen on a TCP port
skry find proc <NAME> <TARGETS>...     which hosts run a process (name or command line)
skry find service <UNIT> <TARGETS>...  systemd unit state across hosts
skry security <TARGETS>...             security pulse report
skry snapshot <TARGETS>... [--out DIR] [--post]
                                       incident snapshot as Markdown + JSON
skry serve <TARGETS>... [--bind ADDR]  web dashboard and /metrics
skry script [--all]                    print the remote script (nothing is run)
skry config path|example|check         config file helpers
```

Targets are names from `~/.ssh/config`, `user@host:port` (IPv6 as
`[2001:db8::1]:22`), or `@group` from the config file.

| Global option | Meaning |
| --- | --- |
| `--interval <SECS>` | Seconds between samples (default 2). |
| `--accept-new` | Record unknown host keys instead of refusing them. Changed keys are always refused. |
| `-c, --config <FILE>` | Config file (also `SKRY_CONFIG`). |
| `--json` | Machine-readable output for query and report commands. |
| `--ssh-config <FILE>` | SSH client config instead of `~/.ssh/config`. |
| `--concurrency <N>` | Maximum hosts contacted at the same time (default 32). |
| `--no-history` | Do not record history. |
| `--no-agent` | Do not use ssh-agent. |
| `--log-file <FILE>` | Write logs to a file (the TUI never logs to the terminal). `-v`/`-vv` for more detail, `SKRY_LOG` for a custom filter. |

**Exit status:** `0` on success, `1` on errors (bad arguments, config
problems), `2` when at least one host could not be reached (`--once`, `find`,
`security`, `snapshot`).

## Terminal UI

| Key | Action |
| --- | --- |
| `←↑↓→` / `hjkl` | Move between tiles |
| `Enter` | Host details |
| `Tab` / `Shift-Tab` / `1`–`5` | Switch panel: overview, processes, network & disks, services & containers, security |
| `n` / `p` | Next / previous host in the details view |
| `/` | Filter by host name or group |
| `f` | Status filter: all, problems, unreachable |
| `o` | Sort: status, name, CPU, memory, disk |
| `S` | Security pulse |
| `[` / `]` | Timeline one minute back / forward |
| `{` / `}` | Timeline 15 minutes back / forward |
| `L` / `End` | Back to live |
| `s` | Incident snapshot of the visible hosts (or the open host) |
| `?` | Help |
| `q` / `Ctrl-C` | Quit |

## Configuration

The config file is optional. skry looks for `~/.config/skry/config.toml`
first (on every platform), then the platform config directory:

| Platform | Path |
| --- | --- |
| Linux | `~/.config/skry/config.toml` |
| macOS | `~/Library/Application Support/skry/config.toml` |
| Windows | `%APPDATA%\skry\config\config.toml` |

`skry config path` shows which file is used, `skry config example` prints a
fully commented example ([docs/config.example.toml](docs/config.example.toml)),
and `skry config check` validates yours.

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

| Key | Default | Meaning |
| --- | --- | --- |
| `interval` | `2.0` | Seconds between samples. |
| `slow_interval` | `60.0` | Seconds between collections of ports, containers, units, logins and updates. |
| `concurrency` | `32` | Hosts contacted at the same time. |
| `connect_timeout` | `10.0` | Seconds for TCP + SSH handshake + authentication (per hop). |
| `command_timeout` | `30.0` | Seconds for one collection command. |
| `thresholds.{cpu,memory,disk,load_per_core}` | 80/95, 85/95, 80/90, 1.5/3.0 | `warning` and `critical` limits. Disk is the fullest real filesystem; load is the 1-minute load divided by cores. |
| `history.enabled` | `true` | Record samples locally. |
| `history.retention_hours` | `24` | How long history is kept. |
| `history.detail_interval` | `30` | Seconds between full host records (summaries are stored every tick). |
| `history.path` | data dir | SQLite database path. |
| `baseline.enabled` | `true` | Statistical baselines. |
| `baseline.alpha` | `0.02` | EWMA smoothing factor per sample. |
| `baseline.z_threshold` | `3.5` | z-score above which a value is a deviation. |
| `baseline.warmup` | `300` | Samples before a baseline is trusted. |
| `baseline.hourly` | `true` | Separate baselines per hour of day. |
| `alerts.cooldown` | `900` | Seconds between notifications for the same alert. |
| `alerts.notify_resolved` | `true` | Notify when an alert clears. |
| `alerts.unreachable_after` | `60` | Seconds of failure before an unreachable alert. |
| `[[webhooks]]` | none | `kind` (`slack`, `discord`, `generic`), `url`, optional `name`, `events`, `snapshot`. |
| `security.failed_login_threshold` | `50` | Failed SSH logins per 24 h that count as a finding (10× is critical). |
| `security.rpm_updates` | `false` | Query dnf's cache on RHEL-family hosts (see [compatibility](#compatibility)). |
| `security.tls` | `[]` | TLS endpoints checked from this machine (at start, then hourly while monitoring). |
| `security.tls_warning_days` / `tls_critical_days` | `21` / `7` | Certificate expiry thresholds. |
| `web.bind` | `127.0.0.1:9187` | Dashboard address. |
| `web.token` | none | Access token; required for non-loopback addresses. `SKRY_WEB_TOKEN` overrides it. |
| `snapshot.dir` | current dir | Where snapshot files are written. |
| `groups.<name>.hosts` | | Hosts or `@othergroup`. |
| `groups.<name>.allowed_ports` | | Ports expected to listen; others are highlighted. |
| `groups.<name>.tls` | | TLS endpoints checked when the group is monitored. |

Unknown keys are rejected so typos do not go unnoticed.

## Web dashboard and Prometheus

```sh
skry serve @production                      # http://127.0.0.1:9187/
SKRY_WEB_TOKEN=$(openssl rand -hex 24) skry serve @production --bind 0.0.0.0:9187
```

The dashboard shows the same fleet and host views as the TUI and updates
live over server-sent events. It is read-only: only `GET` routes exist.

Binding to anything other than a loopback address requires an access token.
Clients present it as `Authorization: Bearer <token>`, or open
`http://host:9187/?token=<token>` once (the dashboard then keeps it in an
HttpOnly, SameSite=Strict cookie). Put a TLS-terminating reverse proxy in
front of it when it leaves your machine.

`/metrics` exposes Prometheus gauges such as `skry_up`, `skry_status`,
`skry_cpu_usage_percent`, `skry_memory_usage_percent`,
`skry_filesystem_usage_percent`, `skry_network_receive_bytes_per_second`,
`skry_failed_ssh_logins_24h`, `skry_pending_security_updates` and
`skry_tls_certificate_days_left`:

```yaml
scrape_configs:
  - job_name: skry
    bearer_token: <token>
    static_configs:
      - targets: ["monitor.example.com:9187"]
```

## History, baselines and alerts

**History.** Every tick a small summary per host is written to
`history.db` in the platform data directory; full host records are written
every `detail_interval` seconds. Data older than an hour is folded into
one-minute buckets, older than six hours into five-minute buckets, and
everything beyond `retention_hours` is deleted. Buckets keep the worst status
they contained, so incidents do not disappear when data is downsampled.

**Baselines.** For each host and metric (CPU, memory, load per core, disk,
network in/out), skry keeps an exponentially weighted moving average of the
mean and variance, globally and per hour of day. After `warmup` samples, a
value whose z-score exceeds `z_threshold` is a deviation: a nightly backup
that always pegs the CPU at 03:00 is normal, the same load at noon is not.
Only upward deviations are reported. Baselines persist across restarts.

**Alerts.** Conditions are threshold breaches, baseline deviations, hosts
unreachable for longer than `unreachable_after`, security findings, and TLS
certificates close to expiry. A condition notifies once when it starts, is
not repeated until `cooldown` has passed (unless it escalates from warning to
critical), is suppressed if it flaps back within the cooldown, and sends a
"resolved" message when it clears. Slack and Discord receive a one-line
summary; `generic` webhooks receive JSON:

```json
{"source":"skry","kind":"firing","host":"db-1","alert":"threshold:cpu","category":"threshold","level":"critical","message":"cpu 97.2% ≥ 95.0% (critical)","timestamp":"2026-09-29T12:00:00Z"}
```

## Security model

- **Nothing is installed or written remotely.** skry opens a normal SSH
  session and runs one read-only POSIX `sh` script per tick (see below).
  There are no uploads, no temp files, no background processes left behind,
  and no privilege escalation. The integration tests verify that the
  filesystem of a monitored host is unchanged after collection.
- **No remote command execution feature.** The only inputs that reach the
  remote shell are a random section marker and, for `find service`, a
  systemd unit name validated against `[A-Za-z0-9@._:-]`. `find port` and
  `find proc` filter locally.
- **Host keys.** Keys are checked against your `known_hosts` files, including
  hashed entries, wildcards, `[host]:port` and `@revoked`. Unknown keys are
  refused unless `--accept-new` is given, which records them (like OpenSSH
  `StrictHostKeyChecking=accept-new`). A changed key is always refused. Jump
  hosts are verified the same way.
- **Credentials.** skry uses your ssh-agent and key files. Passphrases are
  requested once, used to decrypt the key in memory, and never stored or
  logged. Password authentication is not supported, so there is nothing to
  store.
- **Secrets in config.** Webhook URLs and the web token are redacted from
  logs and debug output.
- **Web.** Loopback-only unless a token is configured; constant-time token
  comparison; strict Content-Security-Policy; all remote-provided strings are
  rendered as text, never as HTML.
- **Least privilege.** A normal unprivileged account is enough. Some data
  needs group membership: the journal and auth logs need `adm` or
  `systemd-journal`, container stats need access to the Docker or Podman
  socket. Without it, that panel shows `n/a` with the reason.

## What runs on the remote host

`skry script --all` prints the exact script. In short, one command per tick:

| Section | Source |
| --- | --- |
| CPU | `/proc/stat` (per core and total, from deltas) |
| Memory, swap | `/proc/meminfo` |
| Load, uptime | `/proc/loadavg`, `/proc/uptime` |
| Network | `/proc/net/dev` |
| Disk I/O, usage | `/proc/diskstats`, `df -P` |
| Identity | `hostname`, `uname`, `/etc/os-release` |
| Addresses | `ip -o addr` |
| Processes | `/proc/[pid]/stat` for CPU and memory deltas, `ps -o pid=,user=` (BusyBox compatible) |
| Listening ports | `ss -tlnH`, falling back to `netstat -tln` and `/proc/net/tcp` |
| Containers | `docker`/`podman` `ps` and `stats --no-stream` |
| Failed units | `systemctl --failed --plain --no-legend` |
| Failed SSH logins | `journalctl` (sshd, 24 h), or `/var/log/auth.log`, `/var/log/secure`, `/var/log/messages` |
| Pending updates | `apt-get -s dist-upgrade` (simulation, cached lists), `apk version -l '<'`, optionally `dnf -C updateinfo list` |

Slow sections (ports, containers, units, logins, updates, addresses) run
every `slow_interval` seconds, the rest every tick. Every command is guarded
so that a missing tool, a denied permission or a hang (a 10 s `timeout` where
available) turns into `n/a` rather than an error. skry never runs
`apt update`, `dnf makecache` or anything else that changes state.

## Compatibility

| Remote | Tested | Notes |
| --- | --- | --- |
| Debian 12 | yes | |
| Ubuntu 24.04 | yes | |
| Rocky Linux 9 (RHEL, Alma) | yes | Pending updates need `security.rpm_updates = true` and root: `dnf` appends to its own log files even for cache-only queries, which conflicts with skry's no-writes rule, so it is opt-in. |
| Alpine 3.20 (BusyBox) | yes | `netstat` and BusyBox `ps`; no security metadata for updates. |

Parsers are tested against real output captured from each of these in
[`tests/fixtures`](tests/fixtures), and the integration suite runs skry
against SSH servers for all four in Docker, including a ProxyJump bastion.

## Development

```sh
cargo test                                         # unit, parser and TUI snapshot tests
SKRY_DOCKER_TESTS=1 cargo test --test integration_docker   # real SSH servers in Docker
cargo run -- demo                                  # the TUI on a synthetic fleet
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Inspired by rtop

skry is inspired by [rtop](https://github.com/rapidloop/rtop) by RapidLoop
(MIT licensed), which pioneered the idea of monitoring a server over nothing
but SSH. skry is an independent implementation, not a fork, that takes the
idea to fleets.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in skry by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.
