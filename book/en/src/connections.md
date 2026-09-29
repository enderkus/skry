# Connections, failures and reconnects

## One connection per host

skry opens **one** SSH connection per host and keeps it open for as long as
it runs. Every tick uses a new, lightweight *channel* inside that connection.
This is much cheaper than logging in every two seconds and it keeps your
servers' auth logs quiet: one login per skry session, not one per tick.

To notice dead connections (a rebooted server, a dropped VPN), skry sends an
SSH keepalive every 15 seconds. After three unanswered keepalives, or when a
tick fails, the connection is dropped and a new one is made.

## What happens when something fails

```text
            ┌───────────── success ─────────────┐
            ▼                                   │
  [connect] ──fail──▶ wait 1 s ──▶ [connect] ──fail──▶ wait 2 s ──▶ … 4 s, 8 s, 16 s, 32 s, 60 s, 60 s …
                                                          (exponential backoff, capped at 60 s)
```

- After a failed connection attempt, skry waits and tries again: 1 s, 2 s,
  4 s, 8 s, … up to a maximum of **60 seconds** between attempts. As soon as a
  connection succeeds, the waiting time resets to 1 s.
- Problems that cannot fix themselves — a **host key** problem or a
  **configuration** error — are retried only every 60 seconds, so skry does
  not hammer a server that you need to look at.
- If a **tick** fails on an existing connection (timeout, broken pipe), the
  connection is closed and rebuilt with the same backoff.
- Each host retries independently. One broken host never slows down the
  others.

While a host is failing, its tile stays magenta with the reason. When it
recovers, the tile returns to normal on the next tick. If you set up alerts,
an "unreachable" alert is sent only after the host has been failing for
`alerts.unreachable_after` seconds (default 60), so a brief blip does not wake
anyone up.

## Error messages explained

| You see | What it means | What to do |
| --- | --- | --- |
| `cannot resolve web1: …` | The name is not in `~/.ssh/config` and DNS does not know it | Check the spelling, add a `Host` entry, or use the IP |
| `cannot connect to …: Connection refused` | Nothing listens on that port | Is sshd running? Is the port right (`Port` in `~/.ssh/config`)? |
| `… No route to host` / `Network is unreachable` | Routing or VPN problem | Check your network or VPN |
| `timed out while connecting` | No answer within `connect_timeout` (10 s) | Firewall dropping packets, wrong address, host down |
| `timed out while running the collection command` | Logged in, but the script took longer than `command_timeout` (30 s) | Server extremely overloaded; raise `command_timeout` |
| `unknown host key for …` | The server's key is not in `known_hosts` | Verify it and use `--accept-new`, see [Host keys](host-keys.md) |
| `HOST KEY MISMATCH for …` | The key differs from the recorded one | **Stop and investigate**, see [Host keys](host-keys.md) |
| `host key … is marked @revoked` | Your `known_hosts` explicitly revokes this key | Ask whoever manages the server |
| `authentication failed for user@host (tried N keys)` | The server rejected every key | Check `User`, `IdentityFile`, `authorized_keys`; see [Authentication](authentication.md) |
| `… (no usable keys: start ssh-agent or set IdentityFile)` | skry had no key to offer at all | Start your agent (`ssh-add`) or set `IdentityFile` |
| `via jump host bastion: …` | The problem happened on the jump host, not the target | Fix access to the bastion first |
| `remote command failed: …` / `produced no usable output` | Logged in, but the shell could not run the script | Restricted shells (`rbash`, forced commands, `nologin`) cannot be monitored |
| `ProxyJump chain for … is too long or circular` | Jump hosts refer to each other in a loop, or more than 8 hops | Fix `~/.ssh/config` |

## Graceful behaviour on the server side

- If a single command in the script hangs (for example `df` on a stuck NFS
  mount), the 10-second `timeout` kills just that command and its section
  becomes `n/a`; the rest of the tick still arrives.
- If the whole tick is cut off, skry still uses the sections that arrived
  before the cut.
- If skry itself is killed, the server simply sees a closed SSH connection.
  Nothing is left running.
