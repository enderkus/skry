# Parser fixtures

Real output of the skry remote command, captured with the nonce
`0123456789abcdef`.

| Path | Captured from |
| --- | --- |
| `debian/full.txt` | `debian:12` test container, over SSH as the unprivileged `skry` user |
| `ubuntu/full.txt` | `ubuntu:24.04` test container, over SSH as `skry` |
| `rocky/full.txt` | `rockylinux:9` test container, over SSH as `skry` |
| `alpine/full.txt` | `alpine:3.20` test container (BusyBox userland), over SSH as `skry` |
| `debian-systemd/full-root.txt` | Debian 12 booted with systemd, as root: failed units, journal, security update |
| `debian-systemd/full-user.txt` | Same host as an unprivileged user: journal and systemctl not permitted |
| `debian-systemd/logins-authlog.txt` | `logins` section read from rsyslog `/var/log/auth.log` (ISO timestamps) |
| `alpine/logins-messages.txt` | `logins` section read from BusyBox syslogd `/var/log/messages` |
| `alpine/updates-apk.txt` | `updates` section on an outdated `alpine:3.20.0` |
| `rocky/updates-dnf.txt` | `updates` section on Rocky Linux 9 as root with `rpm_updates` enabled |
| `common/containers-docker.txt` | `containers` section against a Docker engine |

The containers for the `full.txt` captures are defined in `tests/docker`.
To refresh a capture, start that fleet and run the command printed by
`cargo run --example dump_script -- 0123456789abcdef` over SSH.
