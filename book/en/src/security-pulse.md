# Security pulse

The security pulse answers four questions for every host:

1. **Is someone trying to break in?** — failed SSH logins in the last 24 hours
2. **Is it patched?** — pending security updates
3. **Is anything listening that should not be?** — TCP ports outside an allowlist
4. **Are my certificates about to expire?** — TLS certificate expiry

You see it in three places: the `S` screen and the Security panel (`5`) of
the terminal UI, the Security tab of the web dashboard, and the
`skry security` command.

```sh
skry security @production
skry security @production --json
```

```text
HOST    PULSE     FAILED LOGINS 24H         UPDATES               LISTENING         FINDINGS
web-1   ok        3 (top 203.0.113.50 ×3)   0 security / 4 total  as allowed
db-1    critical  1840 (top 198.51.100.23 ×1702)  5 security / 23 total  unexpected: 5432  1840 failed SSH logins in 24h …

TLS ENDPOINT          STATUS    EXPIRY              ISSUER
www.example.com:443   ok        expires in 64 days  R11
api.example.com:443   critical  expires in 5 days   R10
```

## Findings and their levels

| Finding | When | Level |
| --- | --- | --- |
| `failed_logins` | at least `security.failed_login_threshold` (default 50) failed SSH logins in 24 h | warning; **critical** at 10× the threshold (500) |
| `security_updates` | at least one pending security update | warning |
| `unexpected_ports` | a TCP port listens on a non-loopback address and is not in the group's `allowed_ports` | warning |
| TLS certificate | invalid, or expires within `tls_critical_days` (7) | critical |
| TLS certificate | expires within `tls_warning_days` (21) | warning |

Findings appear as `⚑` flags on tiles and can send
[alerts](alerts.md) (category `security`). They do **not** change the host's
health colour — a host can be healthy and still need patching.

## Failed SSH logins

Where skry looks, in this order:

1. **The systemd journal** (`journalctl`), for messages from `sshd` and
   `sshd-session` in the last 24 hours — used on any host running systemd.
2. Otherwise the first readable file of `/var/log/auth.log` (Debian, Ubuntu),
   `/var/log/secure` (RHEL family), `/var/log/messages` (Alpine with syslog),
   plus the previous rotated file (`.1`).

What counts: lines with `Failed password`, `Failed publickey`,
`Failed keyboard-interactive` or `Invalid user`. The addresses after `from`
are counted too, and the top sources are shown. (A password attempt against
a non-existent user usually produces two such lines — see
[calculations](calculations.md#failed-ssh-logins).)

**Permissions:** reading other users' journal entries or the auth log needs
membership in `adm` (Debian/Ubuntu) or `systemd-journal`, or `wheel` on some
systems. Without it skry shows
`n/a (logs not readable (needs adm or systemd-journal group))` — it never
pretends there were zero failures. See [Permissions](permissions.md).

## Pending security updates

Read from the **package cache already on the server**; skry never refreshes
it:

| Distribution | Command | Security detection |
| --- | --- | --- |
| Debian, Ubuntu | `apt-get -s dist-upgrade` (simulation) | package comes from a `…-security` suite |
| Alpine | `apk version -l '<'` | not available — security shows `?` |
| RHEL, Rocky, Alma | `dnf -C -q updateinfo list` — **only if** `security.rpm_updates = true` and skry logs in as root | advisory severity ends in `/Sec.` |

Why is dnf off by default? dnf appends to its own log files even for a
cache-only query, which breaks skry's "writes nothing" promise, and it needs
root. If you accept that, enable it in the config.

If the numbers seem too low, the server's cache may be old. On
Debian/Ubuntu the cache is refreshed by `apt-daily.timer`; check with
`ls -l /var/lib/apt/lists`.

## Listening ports and allowlists

skry lists every listening **TCP** socket (from `ss`, `netstat` or
`/proc/net/tcp`). UDP is not included.

To get a finding for unexpected ports, give the group an allowlist:

```toml
[groups.web]
hosts = ["web1", "web2"]
allowed_ports = [22, 80, 443]
```

- Only ports listening on a **non-loopback** address count. A database
  listening on `127.0.0.1:5432` is not exposed and is fine; one listening on
  `0.0.0.0:5432` is flagged.
- Groups without `allowed_ports` have no port policy: ports are listed
  ("no policy") but never flagged.
- A host in several groups may listen on the union of their allowed ports.

## TLS certificate expiry

List HTTPS (or any TLS) endpoints globally or per group:

```toml
[security]
tls = ["www.example.com:443", "mail.example.com:993"]

[groups.web]
tls = ["api.example.com"]        # port 443 if omitted
```

The check runs **from your computer**, not from the servers: skry makes a
real TLS connection, reads the certificate, and validates the chain against
the Mozilla root certificates built into skry (not your operating system's
store). It reports days left, subject, issuer and any validation problem
(expired, wrong name, unknown issuer…).

- In the terminal UI and `serve`, endpoints are checked at start and then
  every hour. `skry security` and `skry snapshot` check them on every run.
- Each check has a 10-second timeout.

> [!NOTE]
> Certificates issued by a **private/internal CA** are reported as invalid,
> because that CA is not among the public roots. The expiry date is still
> shown correctly.
