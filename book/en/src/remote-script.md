# The script that runs on your servers

This page walks through the script skry sends, section by section, so you
know exactly what touches your servers. Print the real thing with
`skry script --all`.

## How the script is sent

skry sends one command over SSH:

```sh
sh -c '...the script...'
```

Wrapping everything in `sh -c` means it runs under the POSIX shell (`/bin/sh`)
even if your login shell is bash, zsh or fish. The script uses only POSIX
features, so it works with `dash` (Debian, Ubuntu), `bash` (RHEL, Rocky) and
BusyBox `ash` (Alpine).

## The preamble

```sh
LC_ALL=C; export LC_ALL
PATH="$PATH:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"; export PATH
T=''; if command -v timeout >/dev/null 2>&1; then T='timeout 10'; fi
m() { printf '\n@@SKRY-3f9c1a2b7d4e8f60:%s@@\n' "$1"; }
```

- `LC_ALL=C` makes every tool print in plain English with `.` as the decimal
  separator, so the output is always parseable.
- `PATH` is extended because non-interactive SSH sessions often have a very
  short `PATH` that lacks `/usr/sbin` (where `ss` lives).
- `T` becomes `timeout 10` if the `timeout` tool exists. It is put in front of
  commands that could hang, like `df` on a dead network mount.
- `m` prints a **marker line** between sections. The long hexadecimal part is
  a random number chosen fresh for **every** tick. Because nobody can guess
  it, no process name, log line or container name on your server can fake a
  section boundary and confuse skry.

## Sections

Each section starts with a marker and then runs its commands. Every command
sends errors to `/dev/null`: if a tool is missing or a file is not readable,
the section is simply empty and skry shows `n/a`.

| Section | Command(s) | What skry uses it for |
| --- | --- | --- |
| `meta` | `hostname`, `uname -r`, `uname -m`, `date`, `getconf PAGESIZE`, `getconf CLK_TCK`, `id -u` | Host name, kernel, architecture, clock, units needed for calculations |
| `stat` | `grep -E '^(cpu\|btime\|procs_)' /proc/stat` | CPU time counters, total and per core |
| `meminfo` | `cat /proc/meminfo` | Memory and swap |
| `loadavg` | `cat /proc/loadavg` | Load averages |
| `uptime` | `cat /proc/uptime` | Uptime, and the precise time between two ticks |
| `netdev` | `cat /proc/net/dev` | Bytes, packets, errors per network interface |
| `diskstats` | `cat /proc/diskstats` | Reads, writes and busy time per block device |
| `df` | `df -P -k -l` (falls back to `df -P -k`) | Filesystem usage |
| `procstat` | `cat /proc/[0-9]*/stat \| awk '…'` | CPU time and memory of every process (only 6 fields per process are kept) |
| `ps` | `ps -A -o pid= -o user=` (BusyBox fallback `ps -o pid,user`) | Which user owns each process |
| `os` (slow) | `cat /etc/os-release` | Distribution name and version |
| `cpuinfo` (slow) | first model line of `/proc/cpuinfo` | CPU model |
| `ipaddr` (slow) | `ip -o addr show` | IP addresses |
| `ports` (slow) | `ss -tlnH`, else `ss -tln`, else `netstat -tln`, else `/proc/net/tcp` | Listening TCP ports |
| `containers` (slow) | `docker`/`podman` `ps -a` and `stats --no-stream` | Containers, their state, CPU and memory |
| `units` (slow) | `systemctl --failed --plain --no-legend` (only if systemd is running) | Failed services |
| `logins` (slow) | `journalctl --since=-24h -t sshd -t sshd-session`, or `auth.log` / `secure` / `messages` | Failed SSH login attempts, summarised on the server with `awk` so only a few lines travel back |
| `updates` (slow) | `apt-get -s dist-upgrade`, or `apk version -l '<'`, or (opt-in) `dnf -C updateinfo list` | Pending updates from **cached** package lists |

## A few details worth knowing

- **`apt-get -s`** means *simulate*. It reads the package lists already on
  disk and prints what an upgrade *would* do. It does not download, lock or
  change anything, and it works as a normal user.
- **Updates are only as fresh as the server's own package cache.** skry never
  runs `apt update`. If the server's cache is a month old, the list of pending
  updates is a month old too. Most distributions refresh the cache
  automatically (for example `apt-daily.timer` on Debian/Ubuntu).
- **dnf is off by default.** Even in cache-only mode, `dnf` writes to its own
  log files under `/var/log`, which conflicts with skry's promise to write
  nothing, and it needs root. You can turn it on with
  `security.rpm_updates = true` if you accept that.
- **Failed logins are summarised on the server.** Under a brute-force attack
  an auth log can have hundreds of thousands of lines. The script counts them
  with `awk` per source address and hour, and only that summary (at most a
  few thousand short lines) is sent back.
- **`systemctl` exit status is checked.** For an unprivileged user without
  D-Bus access, `systemctl --failed` prints nothing and fails. skry notices
  the failure and shows "n/a (systemctl not permitted)" instead of falsely
  claiming "no failed units".
- **The end marker.** The last line of the script prints an `end` marker. If
  it is missing (for example because the command timed out), skry knows the
  output was cut short and still uses the sections that did arrive.

## Queries that are not part of every tick

Some commands send their own tiny scripts:

| Command | Sections |
| --- | --- |
| `skry find port 5432 …` | `meta`, `ports` |
| `skry find proc nginx …` | `meta`, `procstat`, and `ps -A -o pid= -o user= -o args=` (command lines) |
| `skry find service nginx …` | `meta` and `systemctl show -p Id -p LoadState -p ActiveState -p SubState -p UnitFileState -p Description -- 'nginx.service'` |

The service name is the **only** piece of user input that ever reaches the
remote shell, and it is checked first: only letters, digits and `@ . _ : -`
are accepted. `find port` and `find proc` do their matching on your computer.
