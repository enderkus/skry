# How every number is calculated

This page explains every number skry shows, with worked examples. If a value
in skry ever looks different from another tool, the answer is almost always
here.

## Counters and rates: the basic idea

Linux does not tell you "the CPU is 40 % busy right now". It tells you "since
boot, the CPUs spent 1 234 567 ticks working and 9 876 543 ticks idle". Those
numbers only grow. To get "how busy right now", you measure twice and look at
the **difference**:

```text
                 working ticks   idle ticks
tick 1 (t=0 s)       1 000          9 000
tick 2 (t=2 s)       1 080          9 120
difference              80            120      → busy = 80 / (80 + 120) = 40 %
```

The same idea gives network speed (bytes received now − bytes received
before, divided by seconds) and disk speed.

**Time between two ticks** is taken from the server's own `/proc/uptime`,
not from your computer's clock. Network delays therefore do not distort the
rates. (If uptime is not available, skry falls back to its local clock.)

**Counter resets.** If a counter goes *down* (the server rebooted, or an
interface was recreated), skry shows a rate of 0 for that tick instead of a
nonsense value.

## CPU

From the `cpu` lines of `/proc/stat` (all values are ticks):

| skry shows | Formula (differences between two ticks) |
| --- | --- |
| **CPU %** (tile, overview) | (total − idle − iowait) ÷ total |
| user | (user + nice) ÷ total |
| system | (system + irq + softirq) ÷ total |
| iowait (`io`) | iowait ÷ total |
| steal (`st`) | steal ÷ total |
| per core | same formula on each `cpuN` line |

where *total* = user + nice + system + idle + iowait + irq + softirq + steal.

- **CPU % is relative to all cores.** 100 % means every core is fully busy.
  An 8-core server running one busy single-threaded process shows about 12.5 %.
- **iowait counts as idle** for the headline value, because the CPU is free
  to run other work while waiting. It is shown separately so you can spot
  disk bottlenecks.
- **steal** is time your virtual machine wanted to run but the hypervisor
  gave the CPU to someone else. Consistently high steal means a noisy or
  overcommitted host.

## Memory

From `/proc/meminfo`:

| skry shows | Formula |
| --- | --- |
| **used** | MemTotal − MemAvailable |
| **MEM %** | used ÷ MemTotal |
| cache | Buffers + Cached + SReclaimable |
| swap used | SwapTotal − SwapFree |
| swap % | swap used ÷ SwapTotal |

`MemAvailable` is the kernel's own estimate of how much memory could be
given to programs *without swapping* — it includes caches that can be
dropped. So skry's "used" is **memory that is really taken**. That is why
skry often shows a lower number than tools that count cache as used:
Linux deliberately fills free memory with disk cache, and that is healthy.

On very old kernels (before 3.14) `MemAvailable` does not exist; skry then
uses MemFree + Buffers + Cached + SReclaimable as an approximation.

## Disk usage

From `df -P -k` (sizes in 1 KiB blocks):

- **Use %** = used ÷ (used + available) — exactly what `df` prints. Space
  reserved for root (usually 5 % on ext4) is not counted as available, so a
  disk can show 100 % while `df` still reports a little free space for root.
- **DSK on the tile** is the **highest** use % of all real filesystems.

Which filesystems count as "real"? skry hides:

- pseudo filesystems: `tmpfs`, `devtmpfs`, `udev`, `shm`, `proc`, `sysfs`,
  `cgroup`, `cgroup2`, `devpts`, `mqueue`, `efivarfs`, and entries of size 0
- loop devices (`/dev/loop*`, e.g. snap packages)
- mount points under `/proc`, `/sys`, `/dev`, `/run`, `/snap/`,
  `/var/lib/docker/`, `/var/lib/containers/`, `/var/lib/kubelet/`
- duplicates: when the same device is mounted several times (bind mounts),
  only the shortest mount path is shown

`df` is run with `-l` (local filesystems only) when supported, so a dead NFS
server cannot hang the tick. It also runs under `timeout 10` when available.

## Disk I/O

From `/proc/diskstats`, for whole disks only:

| skry shows | Formula |
| --- | --- |
| Read/s, Write/s | sectors difference × 512 bytes ÷ seconds |
| IOPS r/w | completed reads/writes difference ÷ seconds |
| Util % | milliseconds spent doing I/O ÷ elapsed milliseconds |

Partitions (`sda1`, `nvme0n1p2`, `mmcblk0p1`), loop, ram, zram, nbd, cdrom and
floppy devices are hidden, as are disks that have never done any I/O.
Device-mapper (`dm-0`) and software RAID (`md0`) devices are shown.

Util % close to 100 means the device was busy all the time — for a single
spinning disk that is a bottleneck; for fast SSDs and RAID it is only a hint,
since they can serve many requests at once.

## Network

From `/proc/net/dev`, per interface:

- **RX/s, TX/s** = bytes difference ÷ seconds.
- The per-interface list hides `lo` (loopback) and `veth*` (one per container).
- **↓ ↑ on the tile** is the sum over "primary" interfaces only. Container and
  virtualisation plumbing is excluded so that traffic is not counted twice
  (once on the container's virtual interface and again on the physical one):
  `docker*`, `br-*`, `virbr*`, `cni*`, `flannel*`, `cali*`, `vnet*`, `tunl*`,
  `kube-*`, `vxlan*`, `genev*`, `podman*`, `lxc*`.

Values are **bytes** per second, shown with binary units (1 KiB = 1024 bytes).
Divide by 125 000 to get megabits per second (1 MiB/s ≈ 8.4 Mbit/s).

## Load

- **Load average** (1, 5, 15 minutes) is copied straight from `/proc/loadavg`.
  It is the average number of processes that were running or waiting to run
  (including waiting for disk).
- **Load per core** = 1-minute load ÷ number of cores. This is what the
  thresholds use, because a load of 8 is alarming on a 2-core server and
  relaxed on a 32-core one. Rough guide: below 1.0 per core is comfortable,
  sustained values above 1.5–2 mean work is queueing.

## Processes

From `/proc/<pid>/stat` for every process:

| skry shows | Formula |
| --- | --- |
| **CPU%** | (utime + stime) difference ÷ CLK_TCK ÷ seconds × 100 |
| **Mem%** | RSS pages × page size ÷ MemTotal |
| RSS | resident memory in bytes |

- **Process CPU% is relative to one core**, like in `top`: a process using two
  full cores shows 200 %. (The host CPU % above is relative to all cores.)
- New processes show `…` until they have been seen twice.
- The list contains the 12 busiest processes by CPU **plus** the 12 largest by
  memory (duplicates removed), so a memory hog that is idle is still visible.
- The user name comes from `ps`. With BusyBox or very long user names it may
  be shortened or missing.

## Uptime

`/proc/uptime`, shown as `23d 4h`, `5h 12m`, `7m 3s`.

## Failed SSH logins

Lines in the last 24 hours that contain `Failed password`, `Failed publickey`,
`Failed keyboard-interactive` or `Invalid user` (from sshd). Two things to
know:

- A password attempt for a user that does not exist usually produces **two**
  lines ("Invalid user…" and "Failed password for invalid user…") and is
  therefore counted twice. Treat the number as "failed authentication
  events", not "people".
- For log files (not the journal), the 24-hour window is applied per hour, so
  it covers 24–25 hours.

## Pending updates

- **Debian/Ubuntu:** every package `apt-get -s dist-upgrade` would install.
  A package counts as a **security** update when its source is a
  `…-security` suite (e.g. `Debian-Security` or `noble-security`).
- **Alpine:** packages listed by `apk version -l '<'`. Alpine has no security
  metadata, so the security count shows `?`.
- **RHEL family (opt-in):** unique package names in `dnf updateinfo list`;
  security = advisories with a severity ending in `/Sec.`.
