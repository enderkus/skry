# Alerts and webhooks

skry can notify Slack, Discord or any HTTP endpoint when something goes
wrong — and when it is fixed. Alerts are sent by the terminal UI and by
`skry serve` while they run. (One-shot commands do not send alerts.)

## What triggers an alert

| Category | Condition | Level |
| --- | --- | --- |
| `threshold` | a metric reaches its warning or critical threshold | warning / critical |
| `baseline` | a value deviates from the host's baseline | warning |
| `unreachable` | a host has been failing for `alerts.unreachable_after` seconds (default 60) | critical |
| `security` | a security finding (failed logins, security updates, unexpected ports) or a TLS certificate problem | warning / critical |

Each alert has a stable name, for example `threshold:cpu`,
`baseline:memory`, `unreachable`, `security:failed_logins`,
`tls_certificate`.

## The life of an alert

```text
 condition starts ──▶ FIRING notification
 still true, cooldown not over ──▶ (silence)
 gets worse (warning → critical) ──▶ ESCALATED notification, immediately
 still true after the cooldown ──▶ REMINDER notification
 condition clears ──▶ RESOLVED notification
 comes back within the cooldown ──▶ (suppressed: flapping protection)
```

- **Deduplication**: while a condition stays true, you get one notification,
  not one per tick.
- **Cooldown** (`alerts.cooldown`, default 900 s = 15 min): the minimum time
  between two notifications for the same host and alert. It also protects
  against flapping — a disk that goes 89 % → 91 % → 89 % → 91 % does not
  produce a storm.
- **Escalation** always goes through immediately.
- **Resolved** messages are sent only if the firing was actually delivered
  (and `notify_resolved = true`, the default).
- While a host is unreachable, its other alerts are kept as they were, so you
  do not get a flood of misleading "resolved" messages when a server goes
  down.

## Setting up Slack

1. In Slack, create an *Incoming Webhook* for a channel
   (Apps → Incoming Webhooks) and copy its URL.
2. Add to the config file:

   ```toml
   [[webhooks]]
   name = "ops-slack"
   kind = "slack"
   url = "https://hooks.slack.com/services/<your-webhook-path>"
   ```

A message looks like:

```text
🔴 [CRITICAL] db-1: threshold:cpu — cpu 97.2% ≥ 95.0% (critical)
✅ [CRITICAL] db-1: threshold:cpu resolved — cpu 97.2% ≥ 95.0% (critical)
```

## Setting up Discord

In the channel settings, *Integrations → Webhooks → New Webhook*, copy the
URL:

```toml
[[webhooks]]
kind = "discord"
url = "https://discord.com/api/webhooks/<id>/<token>"
```

Messages longer than 2000 characters are shortened (a Discord limit).

## Generic JSON webhook

For your own tools, PagerDuty/Opsgenie bridges, n8n, and so on:

```toml
[[webhooks]]
name = "pager"
kind = "generic"
url = "https://alerts.example.com/skry"
```

skry sends an HTTP `POST` with `Content-Type: application/json`:

```json
{
  "source": "skry",
  "kind": "firing",
  "host": "db-1",
  "alert": "threshold:cpu",
  "category": "threshold",
  "level": "critical",
  "message": "cpu 97.2% ≥ 95.0% (critical)",
  "timestamp": "2026-09-29T12:00:00Z"
}
```

`kind` is one of `firing`, `reminder`, `escalated`, `resolved`. `level` is
`warning` or `critical`. For TLS alerts `host` is `tls:<endpoint>`.

## Choosing what goes where

`events` limits a webhook to some categories (all four if omitted):

```toml
[[webhooks]]
name = "security-team"
kind = "slack"
url = "https://hooks.slack.com/services/<path-1>"
events = ["security"]

[[webhooks]]
name = "on-call"
kind = "generic"
url = "https://alerts.example.com/skry"
events = ["threshold", "unreachable"]
```

## Settings

```toml
[alerts]
cooldown = 900           # seconds between notifications for the same alert
notify_resolved = true   # send "resolved" messages
unreachable_after = 60   # seconds of failure before "unreachable" fires
```

## Good to know

- Delivery failures (network error, HTTP error) are written to the log
  (`--log-file`) and not retried. Webhook URLs are never printed in full;
  logs show only `https://hooks.slack.com/<redacted>`.
- Webhook URLs must start with `http://` or `https://`.
- Each request has a 10-second timeout.
- Alerts need skry to be running. For round-the-clock alerting, run
  `skry serve` as a service (see [Scripting and automation](automation.md)).
