# What happens on every tick

A **tick** is one measurement round. By default there is one tick every
**2 seconds** per host (setting: `interval`, flag: `--interval`).

## Fast and slow sections

Not everything needs to be measured every 2 seconds. Some data changes slowly
or is more expensive to gather, so skry splits the script into two parts:

| Every tick (fast) | Every 60 seconds (slow, setting `slow_interval`) |
| --- | --- |
| Host name, kernel, clock | Operating system name and version |
| CPU counters (`/proc/stat`) | CPU model |
| Memory (`/proc/meminfo`) | IP addresses |
| Load (`/proc/loadavg`) | Listening TCP ports |
| Uptime (`/proc/uptime`) | Docker/Podman containers |
| Network counters (`/proc/net/dev`) | Failed systemd units |
| Disk counters (`/proc/diskstats`) | Failed SSH logins in the last 24 h |
| Disk usage (`df`) | Pending package updates |
| Per-process CPU and memory | |
| Process owners (`ps`) | |

On a "slow" tick, both parts are sent **in the same single script** — there
is still only one command per tick. Between slow ticks the last known slow
values are kept and shown.

## The first seconds after connecting

```text
t = 0 s   connect, verify host key, log in
t ≈ 0 s   tick 1: memory, disk usage, load, processes (memory) appear
          CPU, network rates, disk I/O and process CPU show "n/a" / "…"
t ≈ 1 s   tick 2 (taken early on purpose): rates appear
t ≈ 3 s   tick 3, and from now on every 2 s
```

Rates need two measurements; that is why skry takes the second one after
only one second instead of waiting a full interval.

## Timing and timeouts

| Limit | Default | What happens when exceeded |
| --- | --- | --- |
| `connect_timeout` | 10 s per hop | The connection attempt fails with "timeout"; skry retries later |
| `command_timeout` | 30 s | The tick fails; skry drops the connection and reconnects |
| Per-command `timeout` on the server | 10 s | That one command (e.g. `df` on a stuck NFS mount) is killed; its section shows `n/a` |

Ticks never pile up: if a tick takes longer than the interval, the next one
simply starts a little later.

## Many hosts at once

Each host has its own worker, so ticks on different hosts run in parallel.
The `concurrency` setting (default 32) limits how many hosts are being
connected to or measured **at the same instant**. With 500 hosts and a
concurrency of 32, a tick round is still fast because each measurement takes
only a few milliseconds of server time.

## What is kept after each tick

- The new values replace the old ones in memory; every view updates.
- A one-line summary (status, CPU, memory, disk, load, network) goes into the
  local history database.
- Every 30 seconds (`history.detail_interval`) the full state of the host
  (processes, disks, containers, …) is stored too, for time travel.
- Health, baselines and alert conditions are re-evaluated.
