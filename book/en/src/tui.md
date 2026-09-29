# The terminal UI, screen by screen

Start it with `skry <hosts>`. Press `?` at any time for the key list, and `q`
to quit. The interface has three screens: **Fleet**, **Host details** and
**Security pulse**.

## The frame around every screen

```text
 skry  8 hosts ● 3 ok ● 1 deviation ● 1 warning ● 1 critical ● 2 unreachable  sort:status    LIVE  every 2s
 …
 09:00 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━● now
 ←↑↓→ move  Enter details  / filter  f status  o sort  [ ] time  S security  s snapshot  ? help  q quit
```

- **Top line**: number of hosts, how many are in each status, the current
  sort order, active filters, and on the right either `LIVE every 2s` or
  `HISTORY 2026-09-29 11:13:00 (-47m 0s)` when you are looking at the past.
- **Timeline** (second to last line): from the oldest recorded moment on the
  left to *now* on the right. The dot shows where you are. See
  [History and time travel](history.md).
- **Bottom line**: the keys that work on this screen. Messages (for example
  "snapshot saved to …") appear here too.

## Fleet screen

One tile per host, as many columns as fit your terminal (each tile is 30
characters wide).

```text
┏ db-1 ━━━━━━━━━━━━━ critical ┓
┃CPU ████████████████▉·  94%  ┃
┃MEM ███████████████▉··  88%  ┃
┃DSK ████████████▊·····  71%  ┃
┃LD 26.69 ↓1.1MiB/s ↑317KiB/s ┃
┃✖ 1 unit ⚑ 3 security        ┃
┗━━━━━━━━━━━━━━━ @production ━┛
```

| Element | Meaning |
| --- | --- |
| Border colour and top-right label | Status (green ok, cyan deviation, yellow warning, red critical, magenta unreachable, grey pending) |
| Thick border, highlighted name | The selected tile |
| `CPU`, `MEM`, `DSK` bars | Percent, coloured green / yellow / red by your thresholds; `·` marks the empty part |
| `LD` | 1-minute load average (coloured by load per core) |
| `↓` `↑` | Network received / sent per second (primary interfaces) |
| `◆ memory z5.9` | This value is unusual for this host ([baselines](baselines.md)) |
| `✖ 1 unit` | Failed systemd units |
| `⚑ 3 security` | Security findings (colour = worst level) |
| `@production` | The (first) group the host belongs to |

A failing host shows the reason instead of bars:

```text
╭ staging-1 ──── unreachable ╮
│✖ host key                  │
│unknown host key for        │
│staging-1.example.com …     │
```

If there are more hosts than fit on the screen, the grid scrolls with the
selection and shows `rows 1-4 of 9` in the corner.

**Keys**

| Key | Action |
| --- | --- |
| arrows or `h` `j` `k` `l` | Move the selection |
| `g` / `G` or `Home` | First / last host |
| `Enter` | Open host details |
| `/` | Type a filter (matches host name, group, or the server's hostname); `Enter` applies, `Esc` clears |
| `f` | Status filter: all → problems (anything not ok) → unreachable only |
| `o` | Sort: status (worst first) → name → CPU → memory → disk (highest first) |
| `Esc` | Clear filter and status filter |
| `S` | Security pulse screen |
| `s` | Snapshot of **all hosts currently visible** (respects your filter) |

## Host details screen

```text
 db-1  critical  deploy@10.0.1.12:22
 Rocky Linux 9.4 (Blue Onyx) · 6.1.0-25-amd64 x86_64 · up 23d 2h · 8 cores · AMD EPYC 7B13
 1 Overview │ 2 Processes │ 3 Network & Disks │ 4 Services & Containers │ 5 Security
```

The header shows name, status, address, OS, kernel, architecture, uptime,
number of cores and CPU model. If the host is failing, a box with the error
and since when appears above the panels (the last known data is still shown
below it).

| Key | Action |
| --- | --- |
| `Tab` / `→` | Next panel |
| `Shift-Tab` / `←` | Previous panel |
| `1`–`5` | Jump to a panel |
| `n` / `p` | Next / previous host (in the fleet's current order) |
| `↑` `↓` `PgUp` `PgDn` | Scroll (processes, security) |
| `Esc` / `Backspace` | Back to the fleet |
| `s` | Snapshot of **this host only** |

### 1 Overview

- Bars for **CPU** (with user / system / iowait / steal split), **MEM**
  (used / total), **SWAP**, **DISK** (the fullest filesystem and its mount
  point).
- **LOAD** 1/5/15 minutes, load per core, number of tasks and running tasks.
- **NET** total receive / send rate.
- **Per core**: one small bar per CPU core.
- **Problems**: `▲` threshold breaches, `◆` baseline deviations, `⚑` security
  findings — each as a sentence, e.g. `▲ disk 91.0% ≥ 90.0% (critical)`.
- Right side: **CPU history** and **Memory history** graphs (last 30 minutes
  from the local history), host name, addresses, number of processes, failed
  units, containers, and when the data was sampled.

### 2 Processes

The busiest processes by CPU plus the largest by memory: PID, user, CPU%
(of **one** core, can exceed 100), memory %, RSS, state (`R` running, `S`
sleeping, `D` waiting for disk, `Z` zombie), command name.

### 3 Network & Disks

- **network**: per interface receive/send rate, totals since boot,
  errors/drops.
- **disk I/O**: per whole disk read/write rate, IOPS, utilisation.
- **filesystems**: mount, device, used, size, a usage bar.

### 4 Services & Containers

- **systemd**: failed units with their state and description, "no failed
  systemd units", or why it is not available (not a systemd host, or not
  permitted).
- **containers**: name, image, status, CPU and memory for Docker or Podman;
  stopped containers are shown dimmed.

### 5 Security

- **Listening TCP ports** with address; ports outside the group's allowlist
  are red with "not in allowlist"; loopback-only ports are dimmed.
- **Failed SSH logins (24h)**: total, source, top addresses.
- **Pending updates**: total, security count, package names.

## Security pulse screen

Press `S` from the fleet. One row per host:

| Column | Meaning |
| --- | --- |
| HOST | Host name |
| PULSE | Worst finding level (ok / warning / critical) |
| LOGINS | Failed SSH logins in 24 h |
| SEC/UPD | Pending security updates / all pending updates |
| UNEXPECTED PORTS | Ports outside the allowlist, `ok`, or the listening ports with "(no policy)" |
| FINDINGS | Each finding as a sentence |

Below it, if configured, a **TLS certificates** table: endpoint, status,
expiry, subject, issuer. Press `S` or `Esc` to return. See
[Security pulse](security-pulse.md).

## Help overlay

`?` or `F1` shows all keys. Any key closes it.

## Terminal tips

- Use a terminal with Unicode and 256 colours (any modern terminal; on
  Windows use Windows Terminal).
- The UI adapts to the window size; widen the window to get more columns.
- The UI never writes log messages to the screen. Use `--log-file skry.log`
  (and `-v` or `-vv`) to see what happens behind the scenes.
