# Your first host, step by step

This walkthrough takes you from zero to watching one server. It assumes skry
is installed ([Installation](installation.md)) and that `ssh yourserver`
logs you in without a password ([SSH basics](ssh-basics.md)).

## 1. Start skry

Use the same name you use with `ssh`:

```sh
skry web1
```

or an explicit address:

```sh
skry deploy@203.0.113.10
skry deploy@203.0.113.10:2222     # non-standard port
```

## 2. If you see "unknown host key"

If skry has never seen this server's key (and neither has your `ssh`), the
tile turns purple and shows:

```text
✖ host key
unknown host key for 203.0.113.10 (SHA256:...); verify it,
then rerun with --accept-new
```

This is skry protecting you. Quit with `q`, then either connect once with
`ssh` (and answer "yes"), or tell skry to record the key:

```sh
skry --accept-new web1
```

`--accept-new` only accepts keys for servers that are **not yet** in
`known_hosts`. It never accepts a key that differs from a recorded one.

## 3. Read the tile

After a second or two the tile fills in:

```text
╭ web1 ──────────────────── ok ╮
│CPU ████▋·············  25%    │
│MEM ███████▍··········  41%    │
│DSK █████████▉········  55%    │
│LD 1.52 ↓1.3MiB/s ↑489KiB/s    │
│                               │
╰───────────────── @production ╯
```

| Part | Meaning |
| --- | --- |
| `web1` | The name you typed |
| `ok` (top right) and the border colour | Overall health: ok, deviation, warning, critical, unreachable |
| `CPU` | How busy the processors are, in percent of all cores |
| `MEM` | Memory really in use (caches that Linux can free are **not** counted) |
| `DSK` | The fullest real filesystem, in percent |
| `LD 1.52` | 1-minute load average |
| `↓ ↑` | Network traffic received / sent per second |
| Fifth line | Extra flags: `◆` unusual value, `✖ 1 unit` failed service, `⚑ 2 security` findings |
| `@production` | The group the host came from (if any) |

> [!NOTE]
> CPU and network show `n/a` for the very first second. They are calculated
> from the **difference** between two measurements, so skry needs a second
> measurement before it can show them. It takes that second measurement
> after one second.

## 4. Look inside

Press `Enter`. You now see the host detail view with five panels. Switch
between them with `Tab` or the number keys `1`–`5`:

1. **Overview** — CPU (also per core), memory, swap, disk, load, network,
   history graphs, and a list of any problems.
2. **Processes** — the busiest processes by CPU and by memory.
3. **Network & Disks** — traffic per interface, I/O per disk, usage per
   filesystem.
4. **Services & Containers** — failed systemd units, Docker/Podman
   containers.
5. **Security** — listening ports, failed SSH logins, pending updates.

Press `Esc` to go back to the fleet.

## 5. Quit

Press `q`. skry closes the SSH connection. Nothing remains on the server.

## Just one quick look?

If you only want the numbers once, without the interface:

```sh
skry web1 --once
```

```text
HOST  STATUS  CPU   MEM    DISK   LOAD  UPTIME  DETAILS
web1  ok      3.1%  41.2%  55.0%  0.12  23d 4h  Debian GNU/Linux 12 (bookworm)
```

Add `--json` for machine-readable output ([Command-line reference](cli.md)).

## Next steps

- Watch several servers: [Many hosts and groups](many-hosts.md)
- Understand what skry just did on your server:
  [The big picture](how-it-works.md)
