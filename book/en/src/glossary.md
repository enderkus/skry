# Glossary

**Agent (monitoring)** — a program installed on each server to collect data.
skry is *agentless*: it has none.

**Agent (SSH)** — a program on your computer that holds unlocked SSH keys and
signs logins (`ssh-agent`, Pageant). Different thing, same word.

**Allowlist** — the `allowed_ports` of a group: ports that may listen.

**Bastion / jump host** — a server you must go through to reach others.
→ [Bastions](proxyjump.md)

**Baseline** — what skry has learned is normal for a host and metric.
→ [Baselines](baselines.md)

**Breach** — a metric at or above its warning or critical threshold.

**Cooldown** — minimum time between two notifications for the same alert.

**Counter** — a number that only grows (CPU ticks, bytes received); rates
come from differences between two counters.

**Deviation** — a value whose z-score exceeds the threshold; shown in cyan.

**EWMA** — exponentially weighted moving average: an average where recent
values count more. Used for baselines.

**Finding** — a security issue: many failed logins, pending security
updates, unexpected ports, a TLS problem.

**Fingerprint** — a short form of a host key, e.g. `SHA256:Zm9v…`, used to
compare keys by eye.

**Fleet** — all the hosts you monitor.

**Host key** — the key a server proves its identity with. → [Host keys](host-keys.md)

**`known_hosts`** — the file listing host keys you trust.

**Load average** — average number of processes running or waiting to run over
1, 5 and 15 minutes.

**Loopback** — addresses that only reach the same machine (`127.0.0.1`,
`::1`).

**Marker** — a line with a random value separating the sections of the remote
script's output.

**`n/a`** — not available: missing tool, missing permission, or not
applicable. Never treated as a problem.

**Nonce** — the random value in markers, new for each tick.

**POSIX sh** — the standard shell language every Unix-like system supports;
skry's script uses only this.

**`/proc`** — a virtual directory where the Linux kernel publishes live
system information as text files.

**ProxyJump** — the SSH setting that connects through a bastion.

**Probe** — a data item that may be unavailable on some hosts (ports,
containers, units, logins, updates).

**RSS** — resident set size: the memory a process actually occupies in RAM.

**Slow sections** — data collected every 60 s instead of every tick.

**Snapshot** — a Markdown + JSON record of the fleet at one moment.

**SSE (server-sent events)** — how the dashboard receives live updates.

**Steal** — CPU time a virtual machine wanted but the hypervisor gave to
someone else.

**Threshold** — a warning or critical limit for CPU, memory, disk or load per
core.

**Tick** — one measurement round for one host (every 2 s by default).

**Tile** — one host's box on the fleet screen.

**z-score** — how many "usual variations" a value is above the usual value.
