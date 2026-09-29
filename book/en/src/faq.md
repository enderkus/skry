# Frequently asked questions

## General

**Do I really not need to install anything on the servers?**
Yes. skry only needs sshd and a POSIX shell, which every Linux server already
has. Nothing is copied, installed or left running.

**Does skry write anything on my servers?**
No. The script only reads. The one thing that is written is outside skry's
control: sshd logs your login, as it does for every SSH login. If you enable
`rpm_updates`, dnf writes its own log files — which is why that option is off
by default.

**Does skry need root or sudo?**
No. A normal account works. Some data needs a group membership (logs,
Docker). → [Permissions](permissions.md)

**Can skry run commands on my servers for me?**
No, deliberately. skry is read-only and has no feature to run arbitrary
commands across the fleet.

**Which servers can I monitor?**
Linux servers reachable over SSH: Debian, Ubuntu, RHEL/Rocky/Alma, Alpine
and their relatives. → [Compatibility](compatibility.md)

**Can I monitor Windows or BSD servers?**
No. skry reads Linux's `/proc`.

**Where does skry run?**
On your computer: Linux, macOS or Windows.

**How is this different from Prometheus + node_exporter, Zabbix, Netdata…?**
Those install an agent on each server and store data centrally for a long
time. skry installs nothing, starts in seconds, and focuses on the live
picture plus 24 hours of history. They complement each other: skry can even
expose `/metrics` for Prometheus.

**Is skry free?**
Yes, open source under MIT or Apache-2.0, at your choice.

## Connecting

**It asks for my key passphrase every time.**
Load the key into your SSH agent (`ssh-add`); skry then never asks.

**Can I use a password instead of keys?**
No. Password logins cannot be automated safely and skry never stores
passwords. Set up a key (`ssh-keygen`, `ssh-copy-id`). → [SSH basics](ssh-basics.md)

**Does skry use my `~/.ssh/config`?**
Yes: `Host`, `HostName`, `User`, `Port`, `IdentityFile`, `IdentitiesOnly`,
`ProxyJump`, `Include`, `UserKnownHostsFile`, `HostKeyAlias`,
`ConnectTimeout`. Other options are ignored.

**Is `ProxyCommand` supported?**
No, use `ProxyJump`. → [Bastions](proxyjump.md)

**Is `--accept-new` safe?**
It is as safe as answering "yes" the first time `ssh` asks. It never accepts
a *changed* key. For full safety compare the fingerprint with the server.

**How many SSH logins does skry produce?**
One per host per skry session (plus reconnects after failures) — not one per
tick.

**Does skry work through a SOCKS proxy or HTTP proxy?**
Not directly. Use a bastion with `ProxyJump`, or a VPN.

## Data and numbers

**Why do CPU and network show `n/a` for a second?**
They are rates: skry needs two measurements. → [Every tick](tick.md)

**Why is my memory usage lower than in other tools?**
skry does not count reclaimable cache as used. → [calculations](calculations.md#memory)

**Is CPU % per core or for the whole machine?**
The host CPU % covers all cores (100 % = all busy). Process CPU % is per core
(like `top`).

**Why is UDP missing from listening ports?**
skry checks TCP listeners only.

**How fresh is the pending-updates list?**
As fresh as the server's own package cache; skry never runs `apt update`.

**Why does a container I run show odd CPU/memory numbers?**
If you monitor a container, the numbers mostly describe the host kernel.
Monitor the host. → [Compatibility](compatibility.md#containers-and-vms-as-targets)

**How often is data collected?**
Every 2 s by default; ports, containers, units, logins and updates every
60 s. → [Every tick](tick.md)

## History, baselines, alerts

**Where is history stored and how big is it?**
In a SQLite file on your computer, about 1–3 MB per host for 24 hours.
→ [History](history.md)

**Can I keep history longer than 24 hours?**
Yes: `history.retention_hours`. For long-term storage, scrape `/metrics` with
Prometheus.

**Does skry send alerts when it is not running?**
No. Run `skry serve` as a service for continuous alerting.
→ [Automation](automation.md)

**Why did I get no "resolved" message?**
Either `notify_resolved = false`, or the original alert was suppressed by the
cooldown (then there is nothing to resolve).

**What is a "deviation"?**
A value that is unusual for that specific host, based on what skry learned.
→ [Baselines](baselines.md)

## Web and Prometheus

**Can colleagues open the dashboard?**
Yes, bind to a network address and set a token. Put it behind TLS.
→ [Web](web.md)

**Can I change settings from the dashboard?**
No. It is read-only.

**Does `/metrics` need the dashboard?**
It is part of `skry serve`; both come together.

## Privacy

**Does skry send data anywhere?**
Only where you tell it: your webhooks. There is no telemetry. The TLS check
connects to the endpoints you configure; the installer contacts GitHub.
