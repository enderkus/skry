# Troubleshooting

Start here: **does plain SSH work?**

```sh
ssh -o BatchMode=yes web1 true && echo works
```

`BatchMode=yes` makes `ssh` behave like skry (no password prompts). If this
fails, fix SSH first; skry cannot do better than `ssh`. Then find your
symptom below.

To see what skry is doing, run with a log file:

```sh
skry -vv --log-file /tmp/skry.log web1
```

## Connecting

**"unknown host key for …"**
The server is not in `known_hosts`. Verify the fingerprint and run once with
`--accept-new`, or connect once with `ssh`. → [Host keys](host-keys.md)

**"HOST KEY MISMATCH for …"**
The server's key changed. Find out why before doing anything (reinstall?
reused IP? attack?). Then `ssh-keygen -R <host>` and reconnect.
→ [Host keys](host-keys.md#host-key-mismatch--what-now)

**"authentication failed for user@host (tried N keys)"**
- Is the user right? Compare with the user `ssh` uses (`ssh -G web1 | grep '^user '`).
- Is your public key in that user's `~/.ssh/authorized_keys` on the server?
- Is the key loaded? `ssh-add -l` lists agent keys.
- Is the key passphrase-protected and skry runs without a terminal (cron,
  CI)? Load it into an agent.
→ [Authentication](authentication.md)

**"… server closed the connection after N keys"**
Your agent offered too many keys and the server's `MaxAuthTries` ran out.
Set `IdentityFile` and `IdentitiesOnly yes` for that host.

**"no usable keys: start ssh-agent or set IdentityFile"**
skry found no key at all. Check that `~/.ssh/id_ed25519` (or your configured
key) exists, or start an agent and `ssh-add`.

**"timed out while connecting"**
A firewall silently drops the traffic, the address is wrong, or the host is
down. Try `nc -vz <host> 22`. Behind a slow VPN, raise `connect_timeout`.

**"cannot connect … Connection refused"**
Nothing listens on that port. Is sshd running? Is the port correct?

**"cannot resolve …"**
The name is neither in `~/.ssh/config` nor in DNS. Use `--ssh-config` if
your SSH config is elsewhere.

**"via jump host bastion: …"**
The error happened on the bastion. Check `ssh bastion` first.

**"remote command failed" / "produced no usable output"**
You logged in, but the script could not run: restricted shell (`rbash`),
`nologin`, a forced `command=` in `authorized_keys`, or a login banner script
that exits. Give the account a normal shell.

## Data

**CPU and network show `n/a` right after start**
Normal: rates need two samples. They appear after about one second.

**A panel says `n/a (…)`**
The reason is in the parentheses. Common ones:

| Message | Fix |
| --- | --- |
| `n/a (logs not readable (needs adm or systemd-journal group))` | Add the monitoring user to `adm`/`systemd-journal` |
| `n/a (no journal or auth log)` | Host has neither journald nor a readable auth log file |
| `n/a (systemctl not permitted)` | Non-root without D-Bus access; nothing to fix unless you use root |
| `n/a (no systemd)` | Host does not use systemd (e.g. Alpine) |
| `n/a (docker: permission denied)` | User is not allowed to use the Docker socket ([why that matters](permissions.md)) |
| `n/a (no docker or podman)` | Neither is installed |
| `n/a (requires root on RPM systems)` | dnf update checks need root |
| `n/a (disabled on RPM systems; set security.rpm_updates)` | Opt in with `rpm_updates = true` |
| `n/a (no supported package manager)` | Not apt, apk or dnf |

**Memory looks lower than in `free` / `htop`**
skry counts memory that is really used (total − available). Caches that
Linux frees on demand are not counted. → [calculations](calculations.md#memory)

**Disk shows a different value than I expected**
The tile shows the **fullest** real filesystem. Pseudo filesystems, snaps,
Docker layers and bind-mount duplicates are hidden. → [calculations](calculations.md#disk-usage)

**Network numbers are lower than the NIC speed test**
skry shows **bytes** per second (MiB/s). Multiply by ~8.4 for Mbit/s.

**Pending updates look outdated**
skry reads the server's cached package lists and never refreshes them.
Check when the cache was last updated (`ls -l /var/lib/apt/lists`).

**A process shows more than 100 % CPU**
Process CPU% is per core (like `top`); 250 % = two and a half cores.

**Failed logins seem doubled**
One attempt on a non-existent user produces two log lines. The number counts
events, not attempts.

**Everything is cyan (deviation)**
Baselines are still learning or your fleet genuinely changed. Raise
`baseline.z_threshold`, or disable baselines. → [Baselines](baselines.md)

## Terminal UI

**Boxes and bars look broken**
Use a UTF-8 terminal and a font with box-drawing characters. On Windows use
Windows Terminal. Check that `LANG` is a UTF-8 locale.

**Colours are wrong / hard to read**
skry uses the terminal's standard colours; choose a different terminal theme.

**`[` says "no history recorded yet"**
History starts when skry starts. Or it is disabled (`--no-history`,
`history.enabled = false`).

**The screen is garbled after a crash**
Run `reset` in the terminal.

## Web dashboard

**"refusing to bind to non-loopback address … without an access token"**
Set `SKRY_WEB_TOKEN` or `[web] token`. → [Web](web.md#access-from-other-machines-the-token-rule)

**Browser shows "access token required"**
Open the URL once with `?token=<token>`.

**Dashboard shows "reconnecting…"**
skry serve stopped or the network dropped; the page reconnects by itself.

## Alerts

**No messages arrive**
- Alerts are only sent while the TUI or `skry serve` runs.
- Check `events` on the webhook.
- Look for `webhook …: HTTP 4xx/5xx` in the log (`--log-file`).
- Remember the cooldown (default 15 minutes) and `unreachable_after`.

## Still stuck?

Open an issue with the output of `skry --version`, your OS, the remote
distribution, and relevant lines from `skry -vv --log-file …` (remove
secrets): <https://github.com/enderkus/skry/issues>
