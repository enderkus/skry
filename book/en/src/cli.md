# Command-line reference

```text
skry [OPTIONS] <TARGETS>...                  monitor in the terminal UI
skry [OPTIONS] <TARGETS>... --once [--json]  one sample, then exit
skry find port <PORT> <TARGETS>...           who listens on a TCP port
skry find proc <NAME> <TARGETS>...           who runs a process
skry find service <UNIT> <TARGETS>...        systemd unit state everywhere
skry security <TARGETS>...                   security pulse report
skry snapshot <TARGETS>... [--out DIR] [--post]
skry serve <TARGETS>... [--bind ADDR]        web dashboard + /metrics
skry script [--all]                          print the remote script
skry config path | example | check           config helpers
skry demo                                    the UI with an imaginary fleet
skry --help | --version
```

## Targets

Everywhere `<TARGETS>` appears you can mix: names from `~/.ssh/config`,
`user@host`, `user@host:port`, `[ipv6]:port`, `ssh://user@host:port` and
`@group`. See [Many hosts and groups](many-hosts.md).

## Global options

These work with every command and may be placed before or after it.

| Option | Meaning |
| --- | --- |
| `--interval <SECS>` | Seconds between ticks (default 2, minimum 0.2) |
| `--accept-new` | Record unknown host keys instead of refusing ([Host keys](host-keys.md)) |
| `-c, --config <FILE>` | Use this config file (also env `SKRY_CONFIG`) |
| `--json` | JSON output for `--once`, `find`, `security`, `snapshot` |
| `--ssh-config <FILE>` | Use this SSH client config instead of `~/.ssh/config` |
| `--concurrency <N>` | Hosts contacted at the same time (default 32) |
| `--no-history` | Do not record history |
| `--no-agent` | Do not use the SSH agent |
| `--log-file <FILE>` | Append logs to this file |
| `-v`, `-vv` | More log detail (info, debug) |

Environment variables: `SKRY_CONFIG` (config file), `SKRY_WEB_TOKEN` (web
token), `SKRY_LOG` (log filter, e.g. `skry=debug`).

## Exit status

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Error: bad arguments, invalid config, unknown group, invalid unit name, … |
| 2 | At least one host could not be reached (`--once`, `find`, `security`, `snapshot`) |

`skry` with no targets prints the help and exits with 1.

## `skry <targets>` — monitor

Opens the [terminal UI](tui.md). Runs until you press `q`. Records
[history](history.md) and sends [alerts](alerts.md).

## `--once` — one sample

```sh
skry @production --once
skry @production --once --json
```

Connects to every host, takes **two** measurements one second apart (so rates
are included), prints, disconnects. `--json` alone (without `--once`) behaves
the same.

Table columns: HOST, STATUS, CPU, MEM, DISK, LOAD, UPTIME, DETAILS (the
error, the breaches and findings, or the OS name).

JSON: an array with one object per host:

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
- `conn`: `connected` or `failed`; when failed, `error` is
  `{"kind": "...", "message": "...", "since": "..."}` with kind one of `auth`,
  `host_key`, `timeout`, `network`, `protocol`, `config`.
- Values that may be unavailable are wrapped:
  `{"status":"ok","value":…}`, `{"status":"unavailable","value":"reason"}` or
  `{"status":"pending"}`.
- Sizes are bytes, rates are bytes per second, percentages are 0–100.

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
(`listening` is `null` when unknown).

## `find proc`

```sh
skry find proc nginx @web
```

Matches the name **case-insensitively** against the process name and its
full command line, on your computer. Lists PID, user and command line of
every match, or `not running`.

JSON: `[{"host": "web1", "matches": [{"pid": 812, "user": "root", "args": "nginx: master process /usr/sbin/nginx"}], "error": null}]`

## `find service`

```sh
skry find service nginx @web          # .service is added if no suffix
skry find service certbot.timer @web
```

```text
HOST  STATE           ENABLED   DESCRIPTION
web1  active/running  enabled   A high performance web server
web2  inactive/dead   disabled  A high performance web server
web3  n/a                       n/a (no systemd)
```

Only `A-Z a-z 0-9 @ . _ : -` are allowed in the name; anything else is
rejected before connecting. A unit that does not exist shows `not installed`.

## `security`

See [Security pulse](security-pulse.md). JSON:
`{"generated": "...", "hosts": [...], "tls": [...]}`.

## `snapshot`

See [Incident snapshots](snapshots.md). Options: `--out DIR`, `--post`.

## `serve`

See [Web dashboard and Prometheus](web.md). Option: `--bind ADDR`
(default `127.0.0.1:9187` or `web.bind`).

## `script`

Prints the POSIX sh script skry would run on each host — without connecting
anywhere. `--all` includes the slow sections. The random marker is shown as
`<random-per-run>`.

## `config`

| Subcommand | Does |
| --- | --- |
| `skry config path` | Prints the config file in use, with "(not created yet)" if it does not exist |
| `skry config example` | Prints a fully commented example config |
| `skry config check` | Loads and validates the config; prints a short summary or the error |

Create your first config:

```sh
mkdir -p ~/.config/skry
skry config example > ~/.config/skry/config.toml
```

(Then delete the example groups and webhooks you do not need.)

## `demo`

Opens the terminal UI with a built-in imaginary fleet that changes over time.
No SSH connections are made. Great for learning the keys.
