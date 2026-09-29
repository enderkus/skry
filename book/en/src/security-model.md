# Security model

This page lists what skry protects against, how, and where the limits are.

## Promises and how they are kept

**Nothing is installed or written on your servers.**
skry sends one read-only POSIX shell script per tick over a normal SSH
session. It does not upload files, create temporary files, start background
processes or escalate privileges. The integration tests check this: they
record every file modified on a test server, run several collections, and
verify the list is unchanged. (Exceptions you control: sshd logging your
login, and dnf's own logs if you enable `rpm_updates`.)

**No way to run arbitrary commands.**
skry has no "run this command on all hosts" feature, on purpose. The only
user-supplied text that ever reaches a remote shell is the unit name in
`skry find service`, validated against `[A-Za-z0-9@._:-]` before anything is
sent. Everything else (`find port`, `find proc`) is filtered on your
computer.

**Section boundaries cannot be forged.**
The markers separating the script's sections contain a random 64-bit value
chosen fresh for every tick. A malicious process name, log line or container
name on the server cannot inject a fake section.

**Server identity is verified.**
Host keys are checked against `known_hosts` (hashed entries, wildcards,
`@revoked`, custom files and `HostKeyAlias` supported). Unknown keys are
refused unless `--accept-new` is given; changed keys are always refused. Jump
hosts are verified the same way. See [Host keys](host-keys.md).

**No passwords.**
Only public-key authentication is supported. Key passphrases are asked once,
used to decrypt the key in memory, and never stored or logged.

**Your keys stay with you.**
ProxyJump uses TCP forwarding through the bastion; the target session is
end-to-end encrypted between you and the target. Agent forwarding is never
used.

**Secrets are redacted.**
Webhook URLs and the web token never appear in logs or debug output; logs
show `https://hooks.slack.com/<redacted>`.

**The web dashboard is locked down by default.**
It listens on `127.0.0.1` only. Listening elsewhere requires a token. Token
comparison is constant-time. Only `GET` routes exist. Responses carry a
strict Content-Security-Policy (`default-src 'self'`), `X-Frame-Options: DENY`,
`X-Content-Type-Options: nosniff` and `Referrer-Policy: no-referrer`. All
text coming from servers (process names, container names, messages) is
inserted as text, never as HTML, so a malicious name cannot run script in
your browser.

## Threats and limits

| Threat | Protection | Limit |
| --- | --- | --- |
| Someone impersonates a server | host key verification | If you `--accept-new` a key without checking the fingerprint on first contact, you trust whatever answered at that moment |
| A compromised server feeds fake data | parsers are strict and cannot be tricked into running anything; markers cannot be forged | A compromised server can of course lie about its own metrics |
| Stolen monitoring key | use a dedicated unprivileged account, `restrict` in `authorized_keys` | The key can still log in and read what that account can read |
| Someone on the network reads the dashboard | loopback by default, token required otherwise | skry serves plain HTTP: use a TLS reverse proxy or an SSH tunnel on untrusted networks |
| Local users on your computer | config and history are normal files in your home directory | Protect them with file permissions (`chmod 600 config.toml`) |

## What is stored locally

- The config file (may contain webhook URLs and the web token).
- The history database: metrics, process names and command names, container
  names, failed-login source addresses. Treat it like any monitoring data.
- `known_hosts` entries added by `--accept-new`.
- Snapshot files you create.

See [Files skry keeps on your machine](data.md).

## Reporting a vulnerability

Please report security issues privately through
[GitHub security advisories](https://github.com/enderkus/skry/security/advisories/new).
