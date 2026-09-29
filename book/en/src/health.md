# Health, colours and status

Every host has exactly one **status**. It decides the colour of the tile and
the order when sorting by status.

| Status | Colour | Meaning |
| --- | --- | --- |
| `unreachable` | magenta | skry cannot connect or cannot read the host |
| `critical` | red | at least one metric is at or above its **critical** threshold |
| `warning` | yellow | at least one metric is at or above its **warning** threshold |
| `deviation` | cyan | all thresholds are fine, but a value is **unusual for this host** (see [Baselines](baselines.md)) |
| `ok` | green | everything within limits |
| `pending` | grey | connected or connecting, first measurement not yet in |

When several things are true at once, the **worst** wins, in the order of the
table: unreachable > critical > warning > deviation > ok.

## Thresholds

Four metrics are compared with static thresholds:

| Metric | What is compared | Warning | Critical |
| --- | --- | --- | --- |
| `cpu` | CPU % (all cores) | 80 | 95 |
| `memory` | memory used % | 85 | 95 |
| `disk` | the fullest real filesystem, % | 80 | 90 |
| `load_per_core` | 1-minute load ÷ cores | 1.5 | 3.0 |

A value **equal to** the limit counts as reaching it (`≥`). Change the
limits in the config file:

```toml
[thresholds.disk]
warning = 85.0
critical = 95.0
```

Both `warning` and `critical` must be given, and `warning` must not be
higher than `critical`.

## What does *not* change the status

- **Security findings** (failed logins, pending security updates, unexpected
  ports) are shown with a `⚑` flag and in the security pulse, and can trigger
  alerts, but they do not turn the tile yellow or red. A server can be
  perfectly healthy and still need patching.
- **Failed systemd units** are shown with a `✖ N unit` flag.
- **Sections that are `n/a`** (for example no permission to read the
  journal) never make a host unhealthy.

## Why is my host "unreachable" when I can ping it?

"Unreachable" means *skry cannot get data*, not only "no network". The tile
tells you why, with one of these labels:

| Label | Meaning |
| --- | --- |
| `unreachable` | network problem: name not found, connection refused, no route |
| `timeout` | no answer in time (firewall dropping packets, overloaded host) |
| `auth failed` | the server rejected all offered keys |
| `host key` | unknown, changed or revoked host key |
| `ssh error` | protocol error, or the remote command produced no usable output |
| `config` | the target could not be resolved (e.g. a ProxyJump loop) |

See [Connections, failures and reconnects](connections.md) and
[Troubleshooting](troubleshooting.md).
