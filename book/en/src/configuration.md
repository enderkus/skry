# Configuration file reference

**You do not need a config file.** skry works out of the box with sensible
defaults. A config file is useful for host groups, alert webhooks, custom
thresholds and web settings.

## Where the file is

skry uses the first of these that applies:

1. The file given with `--config FILE` (or the `SKRY_CONFIG` variable). If
   that file does not exist, skry stops with an error.
2. `~/.config/skry/config.toml` (or `$XDG_CONFIG_HOME/skry/config.toml`) if
   it exists — on **every** platform.
3. The platform's config directory:

| Platform | Path |
| --- | --- |
| Linux | `~/.config/skry/config.toml` |
| macOS | `~/Library/Application Support/skry/config.toml` |
| Windows | `%APPDATA%\skry\config\config.toml` |

If no file exists at the default location, defaults are used silently.

```sh
skry config path       # which file is used
skry config example    # a complete commented example
skry config check      # validate your file
```

## Format

The file uses [TOML](https://toml.io): `key = value` lines, grouped under
`[sections]`. Strings in quotes, lists in brackets, `#` starts a comment.

> [!IMPORTANT]
> **Unknown keys are errors.** A typo like `intervall = 5` or
> `[thresholds.cpus]` stops skry with a message pointing at the line, instead
> of being silently ignored.

## Top-level settings

| Key | Default | Meaning |
| --- | --- | --- |
| `interval` | `2.0` | Seconds between ticks. Minimum 0.2. `--interval` overrides it. |
| `slow_interval` | `60.0` | Seconds between the slow sections (ports, containers, units, logins, updates, OS, IPs). Never shorter than `interval`. |
| `concurrency` | `32` | Hosts contacted at the same time. `--concurrency` overrides it. |
| `connect_timeout` | `10.0` | Seconds for TCP + SSH handshake + login, **per hop**. `ConnectTimeout` in `~/.ssh/config` takes priority for a host. |
| `command_timeout` | `30.0` | Seconds allowed for one tick's command. |

## `[thresholds.*]`

Four tables, each with `warning` and `critical` (both required,
warning ≤ critical). See [Health](health.md).

```toml
[thresholds.cpu]            # CPU busy %, all cores
warning = 80.0
critical = 95.0

[thresholds.memory]         # memory used % (caches not counted)
warning = 85.0
critical = 95.0

[thresholds.disk]           # fullest real filesystem %
warning = 80.0
critical = 90.0

[thresholds.load_per_core]  # 1-minute load ÷ cores
warning = 1.5
critical = 3.0
```

## `[history]`

| Key | Default | Meaning |
| --- | --- | --- |
| `enabled` | `true` | Record history (terminal UI and `serve` only). `--no-history` overrides it. |
| `retention_hours` | `24` | Delete data older than this. |
| `detail_interval` | `30` | Seconds between full host records. |
| `path` | platform data dir | Database file, e.g. `"/var/lib/skry/history.db"`. |

See [History and time travel](history.md).

## `[baseline]`

| Key | Default | Meaning |
| --- | --- | --- |
| `enabled` | `true` | Learn baselines and flag deviations. |
| `alpha` | `0.02` | Smoothing factor per sample, between 0 and 1. |
| `z_threshold` | `3.5` | z-score above which a value is a deviation. |
| `warmup` | `300` | Samples needed before a baseline is used. |
| `hourly` | `true` | Keep a separate baseline for each hour of the day. |

See [Baselines](baselines.md).

## `[alerts]`

| Key | Default | Meaning |
| --- | --- | --- |
| `cooldown` | `900` | Seconds between two notifications for the same alert. |
| `notify_resolved` | `true` | Send a message when an alert clears. |
| `unreachable_after` | `60` | Seconds a host must fail before "unreachable" fires. |

## `[[webhooks]]`

Repeat the block for each target (note the double brackets).

| Key | Required | Meaning |
| --- | --- | --- |
| `kind` | yes | `"slack"`, `"discord"` or `"generic"` |
| `url` | yes | The webhook URL (`http://` or `https://`) |
| `name` | no | A label used in logs |
| `events` | no | Categories to deliver: `"threshold"`, `"baseline"`, `"unreachable"`, `"security"` (all if omitted) |
| `snapshot` | no | `true` to also receive incident snapshots |

See [Alerts and webhooks](alerts.md).

## `[security]`

| Key | Default | Meaning |
| --- | --- | --- |
| `failed_login_threshold` | `50` | Failed SSH logins per 24 h that make a finding (10× = critical). `0` disables the finding. |
| `rpm_updates` | `false` | Query dnf's cache on RHEL-family hosts (needs root, dnf writes its own logs). |
| `tls` | `[]` | TLS endpoints checked from your computer, e.g. `["www.example.com:443"]`. |
| `tls_warning_days` | `21` | Warn when a certificate expires within this many days. |
| `tls_critical_days` | `7` | Critical within this many days (or if invalid). |

See [Security pulse](security-pulse.md).

## `[web]`

| Key | Default | Meaning |
| --- | --- | --- |
| `bind` | `"127.0.0.1:9187"` | Listen address for `skry serve`. `--bind` overrides it. |
| `token` | none | Access token. Required for non-loopback addresses. `SKRY_WEB_TOKEN` overrides it. |

See [Web dashboard](web.md).

## `[snapshot]`

| Key | Default | Meaning |
| --- | --- | --- |
| `dir` | current directory | Where snapshot files are written. `~` is expanded. |

## `[groups.<name>]`

| Key | Meaning |
| --- | --- |
| `hosts` | List of targets: SSH config names, `user@host:port`, or `"@othergroup"` |
| `allowed_ports` | TCP ports this group may listen on; others are flagged. Empty = no policy. |
| `tls` | TLS endpoints checked when this group is monitored |

```toml
[groups.web]
hosts = ["web1", "web2", "deploy@web3.example.com:2222"]
allowed_ports = [22, 80, 443]
tls = ["www.example.com:443"]

[groups.production]
hosts = ["@web", "db1"]
```

Use it with `skry @web`, `skry @production`.

## Settings that live elsewhere

- **How to connect** (addresses, users, ports, keys, jump hosts) is not in the
  skry config. It comes from `~/.ssh/config`, so `ssh` and skry always agree.
- **Secrets**: the webhook URLs and the web token are the only secrets in the
  file. Keep it readable only by you (`chmod 600`). skry never prints them in
  logs.
