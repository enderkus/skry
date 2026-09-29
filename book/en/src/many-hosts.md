# Many hosts and groups

## Listing hosts on the command line

Give skry as many hosts as you like, mixing styles:

```sh
skry web1 web2 db1 deploy@203.0.113.50 root@[2001:db8::7]:2222
```

Accepted forms:

| Form | Example |
| --- | --- |
| A name from `~/.ssh/config` | `web1` |
| A plain address (your local user name, port 22) | `203.0.113.10` |
| user and address | `deploy@203.0.113.10` |
| user, address and port | `deploy@203.0.113.10:2222` |
| IPv6 with a port | `root@[2001:db8::7]:2222` |
| IPv6 without a port | `root@2001:db8::7` |
| URI style | `ssh://deploy@203.0.113.10:2222` |
| A group from the config file | `@production` |

Whatever you write explicitly (user, port) wins over `~/.ssh/config`.
The same host given twice is only monitored once.

## Groups

Typing twenty names every time is no fun. Put them in a group in the skry
configuration file (see [Configuration](configuration.md) for where the
file lives; `skry config path` prints it):

```toml
[groups.web]
hosts = ["web1", "web2", "web3"]

[groups.databases]
hosts = ["db1", "db2", "deploy@10.0.5.20:2222"]

[groups.production]
hosts = ["@web", "@databases", "cache1"]
```

Now:

```sh
skry @web            # three hosts
skry @production     # seven hosts: web1-3, db1-2, the 10.0.5.20 one, cache1
skry @web db-test    # groups and single hosts can be mixed
```

Groups can contain other groups (with `@`). A group that includes itself,
directly or indirectly, is reported as an error instead of looping forever.

## What else groups do

A group can also carry a **policy**:

```toml
[groups.web]
hosts = ["web1", "web2"]
allowed_ports = [22, 80, 443]          # anything else listening is flagged
tls = ["www.example.com:443"]          # certificate expiry is checked
```

- `allowed_ports` — in the [security pulse](security-pulse.md), a server in
  this group that listens on another port (on a non-loopback address) is
  flagged. If a host belongs to several groups, the allowed ports of all of
  them are combined.
- `tls` — these HTTPS endpoints are checked whenever the group is monitored.

## How many hosts can skry handle?

skry contacts at most 32 hosts **at the same time** by default (the
`concurrency` setting). This is a limit on simultaneous work, not on the
number of hosts: with 200 hosts, skry simply works through them in parallel
batches. Each host still keeps its own SSH connection open between ticks.

A slow or broken host never delays the others: every host has its own
independent worker. See [Performance and overhead](performance.md) for sizing.

## Navigating a big fleet in the terminal UI

| Key | Action |
| --- | --- |
| `/` then text | Show only hosts whose name or group contains the text |
| `f` | Cycle: all → only problems → only unreachable |
| `o` | Cycle sorting: status (worst first) → name → CPU → memory → disk |
| `Esc` | Clear filters |

The header always shows the totals, e.g. `12 hosts ● 9 ok ● 1 warning ● 2 unreachable`.
