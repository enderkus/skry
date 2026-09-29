# The big picture

This page explains, without jargon, what happens between typing `skry web1`
and seeing numbers on your screen.

```text
 YOUR COMPUTER                                        YOUR SERVER (web1)
 ┌──────────────────────────────┐                     ┌───────────────────────────┐
 │ skry                         │   one SSH session   │ sshd                      │
 │  1. reads ~/.ssh/config      │ ══════════════════▶ │  (the normal SSH server)  │
 │  2. checks the host key      │                     │                           │
 │  3. logs in with your key    │   every 2 seconds:  │  sh -c '...script...'     │
 │  4. sends one small script   │ ──────────────────▶ │   cat /proc/stat          │
 │                              │                     │   cat /proc/meminfo       │
 │  6. parses the answer        │ ◀────────────────── │   df -P ...               │
 │  7. computes rates & health  │   plain text answer │  (exits, leaves nothing)  │
 │  8. draws the screen         │                     │                           │
 │  9. stores a summary locally │                     └───────────────────────────┘
 └──────────────────────────────┘
```

## Step by step

1. **Resolve the name.** `web1` is looked up in `~/.ssh/config` to find the
   real address, user, port, key files and any jump hosts — exactly like the
   `ssh` command would.
2. **Verify the server.** The server presents its host key. skry compares it
   with your `known_hosts`. Unknown or changed keys stop the connection here
   (see [Host keys](host-keys.md)).
3. **Log in.** skry offers your keys, first from your SSH agent, then from key
   files (see [Authentication](authentication.md)).
4. **Keep the connection open.** This single SSH connection is reused for
   every measurement, so the expensive handshake happens only once.
5. **Every tick** (every 2 seconds by default), skry opens a lightweight
   channel inside that connection and runs **one** shell command. That command
   is a short POSIX `sh` script that prints the contents of a few files from
   `/proc` and the output of a few standard tools, separated by marker lines.
   The script ends, the channel closes. Nothing keeps running on the server.
6. **Parse.** skry splits the text at the markers and reads each part with
   a dedicated parser.
7. **Compute.** Many Linux counters only ever go up ("CPU time spent since
   boot", "bytes received since boot"). skry subtracts the previous
   measurement from the current one and divides by the elapsed time to get
   rates and percentages. It then compares values with your thresholds and
   with the host's own history (baselines).
8. **Show.** The terminal UI, the web dashboard and `/metrics` all read the
   same up-to-date state.
9. **Remember.** A short summary of each measurement goes into a local
   SQLite database on **your** computer, so you can scroll back in time.

## Why one script instead of many commands?

Running ten separate commands would mean ten round trips over the network
and ten processes started by sshd. One script means **one** round trip per
tick, which keeps the load on the server and on the network tiny, and makes
all values in a sample come from the same instant.

## Why plain SSH?

- It is already there on every Linux server.
- It is already secured, audited and allowed through your firewalls.
- It already knows who you are (your keys) and who the server is (host keys).
- Nothing new needs to be opened, installed, updated or trusted.

## What skry does **not** do on the server

- It does not copy any file to the server.
- It does not create temporary files.
- It does not use `sudo` or ask for root.
- It does not start anything that keeps running.
- It does not run package manager commands that change state
  (`apt update`, `dnf makecache`, …).

One honest caveat: like every SSH login, **your login itself is logged by
the server** (for example in `/var/log/auth.log` or the journal). That is sshd
doing its normal job, not skry writing anything.

You can print the exact script at any time — nothing is executed:

```sh
skry script          # the part that runs every tick
skry script --all    # including the slower, occasional sections
```

The next pages go deeper: [every tick](tick.md), [the script](remote-script.md),
[the calculations](calculations.md).
