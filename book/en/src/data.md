# Files skry keeps on your machine

skry writes nothing on your servers. On your own computer it may use these
files:

| File | Purpose | Default location |
| --- | --- | --- |
| Config | your groups, thresholds, webhooks | see [Configuration](configuration.md#where-the-file-is) |
| History database | metrics history and baselines | Linux `~/.local/share/skry/history.db`, macOS `~/Library/Application Support/skry/history.db`, Windows `%APPDATA%\skry\data\history.db` |
| `known_hosts` | host keys you accepted with `--accept-new` | `~/.ssh/known_hosts` (or your `UserKnownHostsFile`) |
| Snapshots | Markdown and JSON reports | current directory, `snapshot.dir`, or `--out` |
| Log file | only if you pass `--log-file` | where you say |

The history database may be accompanied by `history.db-wal` and
`history.db-shm` while skry runs; these are normal SQLite working files.

## Removing everything

```sh
# Linux
rm -rf ~/.local/share/skry ~/.config/skry
# macOS
rm -rf ~/Library/Application\ Support/skry ~/.config/skry
```

On Windows delete `%APPDATA%\skry`. Remove the program itself as described
in [Installation](installation.md#uninstalling). Entries in `known_hosts`
are standard OpenSSH entries; keep them (your `ssh` uses them too) or remove
them with `ssh-keygen -R <host>`.
