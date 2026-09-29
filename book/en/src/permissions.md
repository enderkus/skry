# Permissions and least privilege

skry is designed to run as an **ordinary, unprivileged user**. It never uses
`sudo`. Most data is readable by everyone on Linux; a few items need a group
membership. When something is not permitted, skry shows `n/a (reason)` for
that item and everything else keeps working.

## What needs what

| Data | Needs | Without it |
| --- | --- | --- |
| CPU, memory, load, uptime, network, disk I/O, disk usage | nothing (world-readable `/proc` and `df`) | — |
| Process list with CPU/memory | nothing | With `hidepid=2` on `/proc`, only your own processes are visible |
| Process owners | nothing | — |
| Listening ports | nothing | — |
| IP addresses, OS, kernel | nothing | — |
| Failed systemd units | usually nothing (D-Bus) | `n/a (systemctl not permitted)` on systems without D-Bus access for normal users |
| Failed SSH logins (journal) | group `systemd-journal` or `adm` (sometimes `wheel`) | `n/a (logs not readable …)` |
| Failed SSH logins (`/var/log/auth.log`, `secure`) | group `adm` (Debian/Ubuntu) or root (`secure` on RHEL is root-only) | `n/a (logs not readable …)` |
| Pending updates, Debian/Ubuntu/Alpine | nothing | — |
| Pending updates, RHEL family | root **and** `security.rpm_updates = true` | `n/a (requires root on RPM systems)` or `n/a (disabled on RPM systems; …)` |
| Containers (Docker) | access to `/var/run/docker.sock` (group `docker`) | `n/a (docker: permission denied)` |
| Containers (Podman) | nothing for your own rootless containers | only your user's containers are shown |

## Recommended setup for a monitoring account

Debian / Ubuntu:

```sh
sudo useradd -m -s /bin/sh skry
sudo usermod -aG adm,systemd-journal skry
```

RHEL / Rocky / Alma:

```sh
sudo useradd -m -s /bin/sh skry
sudo usermod -aG systemd-journal skry     # journal (failed logins)
```

Then install your monitoring public key in `~skry/.ssh/authorized_keys` (see
[Authentication](authentication.md)).

> [!WARNING]
> **About the `docker` group:** membership in `docker` is effectively root
> access to the host (anyone in it can start a privileged container). Only
> add the monitoring account to it if you accept that. Without it, skry
> simply shows containers as `n/a`.

## Restricting the monitoring key further (optional)

You can limit what the key may do in `authorized_keys`, as long as normal
command execution stays possible:

```text
restrict,pty ssh-ed25519 AAAA... skry-monitoring
```

`restrict` disables port, agent and X11 forwarding. (skry needs no pty, but
`pty` is harmless; you may drop it.) Do **not** use `command="…"` forced
commands: they replace skry's script and collection fails.

## Why not root?

Logging in as root gives skry nothing essential except dnf update checks
and reading root-only logs, and it means a stolen monitoring key would be a
root key. Prefer the unprivileged account with the groups above.

## What the server sees

For each skry session the server logs **one** SSH login for the monitoring
account (and one logout when skry stops). Every tick runs a handful of
short-lived processes (`sh`, `cat`, `grep`, `awk`, `df`, `ps`, …) as that user.
No files are created.
