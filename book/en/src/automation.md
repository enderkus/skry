# Scripting and automation

## JSON + jq recipes

```sh
# Names of unhealthy hosts
skry @production --once --json | jq -r '.[] | select(.status != "ok") | .name'

# CPU and memory as CSV
skry @production --once --json \
  | jq -r '.[] | [.name, .metrics.cpu.total_pct, .metrics.mem.used_pct] | @csv'

# Hosts with pending security updates
skry security @production --json \
  | jq -r '.hosts[] | select(.updates.status == "ok" and .updates.value.security > 0) | .host'

# Which hosts run redis?
skry find proc redis-server @production --json | jq -r '.[] | select(.matches | length > 0) | .host'
```

## Using the exit status

```sh
# Fail a CI job or a cron check when any host is unreachable
if ! skry @production --once > /tmp/fleet.txt; then
    echo "some hosts are unreachable"; cat /tmp/fleet.txt
fi
```

Exit code 2 means "at least one host unreachable"; 1 means a usage or config
error. See [Command-line reference](cli.md#exit-status).

## Non-interactive runs

When skry runs from cron, CI or systemd:

- There is no terminal to ask for key passphrases. Use an SSH agent, or a
  dedicated key without passphrase restricted to a monitoring account.
- Host keys must already be in `known_hosts` (or run once with
  `--accept-new`).
- Use `--log-file` and `-v` to keep a log.

## Running `skry serve` as a service (Linux, systemd)

`/etc/systemd/system/skry.service`:

```ini
[Unit]
Description=skry fleet monitoring
After=network-online.target
Wants=network-online.target

[Service]
User=monitor
Environment=SKRY_WEB_TOKEN=change-me
ExecStart=/usr/local/bin/skry serve @production --bind 0.0.0.0:9187 --log-file /home/monitor/skry.log
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now skry
```

The `monitor` user needs its own `~/.ssh/config`, key and `known_hosts`
(and `~/.config/skry/config.toml` for groups and webhooks). This gives you
continuous alerting, history and a Prometheus endpoint.

## A daily security report in chat

```sh
# crontab -e
0 8 * * 1-5  /usr/local/bin/skry snapshot @production --post --out /var/tmp/skry >/dev/null 2>&1
```

With a webhook marked `snapshot = true`, the team gets a morning summary.

## Pre-deployment check

```sh
skry find port 8080 @web --json \
  | jq -e 'all(.[]; .listening == false)' > /dev/null \
  || { echo "port 8080 already in use somewhere"; exit 1; }
```
