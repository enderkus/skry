# Performance and overhead

## On your servers

Per tick (default every 2 s) one short script runs. It starts a handful of
tiny processes (`sh`, `cat`, `grep`, `awk`, `df`, `ps`) that finish in a few
milliseconds. There is no resident process between ticks.

Every 60 s the slow sections add a few more commands. The most expensive is
`docker stats --no-stream`, which takes about two seconds on the Docker side
(it samples container usage). On hosts without Docker/Podman it costs nothing.

## On the network

- **One** SSH connection per host, kept open.
- Command sent per tick: about **1.4 KB** (about **4.2 KB** on slow ticks).
- Answer per tick: a few KB of base data plus roughly **60 bytes per running
  process**. A server with 300 processes returns about 20–25 KB per tick,
  i.e. about 10–12 KB/s at the default interval. A small container with a
  handful of processes returns about 5 KB.
- Plus SSH keepalives every 15 s.

To reduce traffic, increase the interval (`--interval 10`).

## On your computer

- One lightweight task per host; parsing a tick takes microseconds.
- Memory grows with the number of hosts and the amount of data per host
  (processes, containers); each host keeps only its latest state in memory.
- History writes happen on a background thread in small transactions.

## Sizing guidelines

| Fleet | Suggested settings |
| --- | --- |
| 1–50 hosts | defaults (2 s interval, concurrency 32) |
| 50–300 hosts | `interval = 5`, `concurrency = 64` |
| 300+ hosts | `interval = 10`–`30`, `concurrency = 64`–`128`; consider splitting into groups watched by different skry instances |

`concurrency` limits simultaneous connects/ticks, not the number of hosts.
Raise it if ticks for a large fleet start to fall behind; lower it if your
bastion or VPN struggles with many parallel connections (every host behind a
bastion is one tunnel through it).

Your file-descriptor limit matters for very large fleets: each host uses at
least one TCP connection (two with a bastion). `ulimit -n` of 1024 is fine for
several hundred hosts.
