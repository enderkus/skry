# History and time travel

When you watch hosts with the terminal UI (`skry <hosts>`) or the web
dashboard (`skry serve`), skry records what it sees in a small database on
**your** computer. You can then scroll back to see how the fleet or a single
host looked earlier.

## Is it always on?

- **On by default** for the terminal UI and `skry serve`.
- **Never** for one-shot commands (`--once`, `find`, `security`,
  `snapshot`) — they do not touch the database.
- Turn it off for one run with `--no-history`, or permanently:

  ```toml
  [history]
  enabled = false
  ```

With history off, everything else still works; you just cannot go back in
time, the history graphs stay empty, and [baselines](baselines.md) start from
scratch every time skry starts.

## What is stored

| What | How often | Contains |
| --- | --- | --- |
| Summary | every tick, per host | status, CPU %, memory %, fullest disk %, load per core, network in/out |
| Full record | every 30 s per host (`history.detail_interval`) | everything you see in the host details: processes, disks, interfaces, containers, units, findings… |
| Baselines | every 5 minutes and when skry exits | the learned "normal" values per host and metric |

Unreachable periods are recorded too, so you can see *when* a host went
away.

## How long it is kept, and how it shrinks

Recent data is detailed, older data is thinned out automatically so the file
stays small:

```text
 now ◀──── last hour ────▶◀──── 1 h … 6 h ────▶◀──── 6 h … 24 h ────▶ deleted
       every tick           1-minute averages      5-minute averages
       full record / 30 s   full record / 5 min    full record / 5 min
```

- Averages keep the **worst** status of their bucket, so a 20-second outage
  is still visible as "unreachable" an hour later.
- Everything older than `history.retention_hours` (default 24) is deleted.
- This maintenance runs every five minutes while skry is running.

## Scrubbing back in the terminal UI

| Key | Action |
| --- | --- |
| `[` / `]` | One minute back / forward |
| `{` / `}` | 15 minutes back / forward |
| `L` or `End` | Back to live |

While you are in the past:

- The header shows `HISTORY 2026-09-29 11:13:00 (-47m 0s)` in yellow, and the
  dot on the timeline moves left.
- **Fleet tiles** show the stored summary for that moment: status, CPU,
  memory, fullest disk, load **per core** and network.
- **Host details** show the nearest full record *at or before* that moment;
  the timeline line says `details recorded 11:12:41` so you know its exact
  time.
- A host with no data at that moment (not yet monitored, or skry was not
  running) shows as pending.
- You cannot go further back than the oldest record, and moving forward past
  *now* returns to live.
- `s` takes a snapshot **of what you are looking at**, so you can document
  the past state of an incident.

Live data keeps being collected in the background while you look at the past.

## Where the file is

| Platform | Default path |
| --- | --- |
| Linux | `~/.local/share/skry/history.db` |
| macOS | `~/Library/Application Support/skry/history.db` |
| Windows | `%APPDATA%\skry\data\history.db` |

Change it with `history.path`. The file is a normal SQLite database; you can
open it with `sqlite3` if you are curious (tables `samples`, `details`,
`baselines`). Deleting it is safe while skry is not running.

## How big does it get?

A summary row is a few dozen bytes; a full record is a few kilobytes. With
the defaults, expect roughly **1–3 MB per host** for 24 hours, depending on
the number of processes and containers.
