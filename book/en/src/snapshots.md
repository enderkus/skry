# Incident snapshots

During an incident you want a record of *what everything looked like right
now*: for the post-mortem, for a ticket, or to hand over to a colleague. A
snapshot captures the complete state of the selected hosts in two files:

- a **Markdown** report, readable by humans (paste it into a ticket, a wiki,
  a chat), and
- a **JSON** file with every value, for tools.

## Taking a snapshot

**In the terminal UI:** press `s`.

- On the fleet screen: all **visible** hosts (your filter applies — filter to
  the affected hosts first, then press `s`).
- On a host's details: only **that host**.
- In [history mode](history.md): the state at the moment you are looking at.

The bottom line confirms: `snapshot of 3 host(s) saved to ./skry-snapshot-20260929-120000.md`.

**From the command line:**

```sh
skry snapshot @production
skry snapshot web1 db1 --out ~/incidents/2026-09-29
skry snapshot @production --post          # also send to webhooks
skry snapshot @production --json          # also print the JSON to stdout
```

The command takes two measurements one second apart (so CPU and network
rates are included), checks the configured TLS endpoints, writes the files
and prints their paths.

## Where the files go

1. `--out DIR` if given,
2. otherwise `snapshot.dir` from the config,
3. otherwise the current directory.

Files are named `skry-snapshot-YYYYMMDD-HHMMSS.md` and `.json` (UTC time).

## What is in the report

- A summary table: every host with status, CPU, memory, disk, load per core,
  uptime and notes (errors, breaches, deviations).
- TLS certificate table (if configured).
- Per host: address, groups, OS, kernel, CPU model, uptime, IP addresses;
  problems (threshold breaches, baseline deviations, security findings);
  CPU split and per-core values; load; memory and swap; filesystems; disk
  I/O; network interfaces; top processes; failed units; containers;
  listening ports; failed SSH logins; pending updates.

## Posting to chat or a webhook

Mark webhooks that should receive snapshots:

```toml
[[webhooks]]
kind = "slack"
url = "https://hooks.slack.com/services/<your-webhook-path>"
snapshot = true
```

- Slack and Discord receive a **short summary** (one line per host with its
  status and main numbers, plus threshold breaches).
- Generic webhooks receive the **complete JSON**:
  `{"source":"skry","kind":"snapshot","snapshot":{…}}`.

In the terminal UI, pressing `s` posts automatically to every webhook with
`snapshot = true`. On the command line, add `--post`.
