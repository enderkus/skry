//! Alert evaluation, deduplication, cooldown and webhook delivery.
//!
//! The engine reports the full set of active conditions for a host after
//! every sample. [`AlertManager`] turns changes in that set into
//! notifications:
//!
//! * a new condition fires once;
//! * a condition that stays active is not repeated until the cooldown has
//!   passed (then a reminder is sent), unless it escalates in severity;
//! * a condition that clears and returns within the cooldown is suppressed
//!   (flapping protection);
//! * a cleared condition sends a "resolved" notification if its firing was
//!   delivered and `notify_resolved` is on.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::{AlertCategory, Webhook, WebhookKind, redact_url};
use crate::model::Level;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct AlertKey {
    pub host: String,
    /// Stable identifier such as `threshold:cpu` or `security:ports`.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Condition {
    pub key: AlertKey,
    pub category: AlertCategory,
    pub level: Level,
    pub message: String,
}

impl Condition {
    pub fn new(
        host: &str,
        name: impl Into<String>,
        category: AlertCategory,
        level: Level,
        message: impl Into<String>,
    ) -> Self {
        Condition {
            key: AlertKey {
                host: host.to_string(),
                name: name.into(),
            },
            category,
            level,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    Firing,
    Reminder,
    Escalated,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Notification {
    pub kind: NotificationKind,
    pub host: String,
    pub alert: String,
    pub category: AlertCategory,
    pub level: Level,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl Notification {
    fn from_condition(kind: NotificationKind, c: &Condition) -> Self {
        Notification {
            kind,
            host: c.key.host.clone(),
            alert: c.key.name.clone(),
            category: c.category,
            level: c.level,
            message: c.message.clone(),
            timestamp: Utc::now(),
        }
    }

    /// One-line human summary used by chat webhooks.
    pub fn text(&self) -> String {
        let icon = match (self.kind, self.level) {
            (NotificationKind::Resolved, _) => "✅",
            (_, Level::Critical) => "🔴",
            (_, Level::Warning) => "🟠",
            (_, Level::Ok) => "🔵",
        };
        let verb = match self.kind {
            NotificationKind::Firing => "",
            NotificationKind::Reminder => " (still active)",
            NotificationKind::Escalated => " (escalated)",
            NotificationKind::Resolved => " resolved",
        };
        format!(
            "{icon} [{}] {}: {}{verb} — {}",
            self.level.as_str().to_uppercase(),
            self.host,
            self.alert,
            self.message
        )
    }
}

#[derive(Debug, Clone)]
struct Active {
    condition: Condition,
    /// When a firing/reminder notification was last delivered for this
    /// activation; `None` if it was suppressed.
    last_sent: Option<Instant>,
}

#[derive(Debug)]
pub struct AlertManager {
    cooldown: Duration,
    notify_resolved: bool,
    active: HashMap<AlertKey, Active>,
    /// Last time any firing notification went out per key, surviving
    /// resolution, for flapping protection.
    last_fired: HashMap<AlertKey, Instant>,
}

impl AlertManager {
    pub fn new(cooldown: Duration, notify_resolved: bool) -> Self {
        Self {
            cooldown,
            notify_resolved,
            active: HashMap::new(),
            last_fired: HashMap::new(),
        }
    }

    /// Replaces the active conditions of `host` and returns the
    /// notifications to deliver.
    pub fn evaluate(
        &mut self,
        host: &str,
        conditions: Vec<Condition>,
        now: Instant,
    ) -> Vec<Notification> {
        let mut out = Vec::new();
        let mut seen = Vec::with_capacity(conditions.len());
        for c in conditions {
            debug_assert_eq!(c.key.host, host);
            seen.push(c.key.clone());
            let in_cooldown = |t: Option<Instant>| {
                t.is_some_and(|t| now.saturating_duration_since(t) < self.cooldown)
            };
            match self.active.get_mut(&c.key) {
                Some(a) => {
                    let escalated = c.level > a.condition.level;
                    if escalated {
                        out.push(Notification::from_condition(
                            NotificationKind::Escalated,
                            &c,
                        ));
                        a.last_sent = Some(now);
                        self.last_fired.insert(c.key.clone(), now);
                    } else if !in_cooldown(a.last_sent.or(self.last_fired.get(&c.key).copied())) {
                        out.push(Notification::from_condition(
                            if a.last_sent.is_some() {
                                NotificationKind::Reminder
                            } else {
                                NotificationKind::Firing
                            },
                            &c,
                        ));
                        a.last_sent = Some(now);
                        self.last_fired.insert(c.key.clone(), now);
                    }
                    a.condition = c;
                }
                None => {
                    let last_sent = if in_cooldown(self.last_fired.get(&c.key).copied()) {
                        None
                    } else {
                        out.push(Notification::from_condition(NotificationKind::Firing, &c));
                        self.last_fired.insert(c.key.clone(), now);
                        Some(now)
                    };
                    self.active.insert(
                        c.key.clone(),
                        Active {
                            condition: c,
                            last_sent,
                        },
                    );
                }
            }
        }
        let cleared: Vec<AlertKey> = self
            .active
            .keys()
            .filter(|k| k.host == host && !seen.contains(k))
            .cloned()
            .collect();
        for key in cleared {
            if let Some(a) = self.active.remove(&key)
                && self.notify_resolved
                && a.last_sent.is_some()
            {
                out.push(Notification::from_condition(
                    NotificationKind::Resolved,
                    &a.condition,
                ));
            }
        }
        out
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[derive(Debug, Serialize)]
struct GenericPayload<'a> {
    source: &'static str,
    #[serde(flatten)]
    notification: &'a Notification,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Builds the JSON body for a chat message.
pub fn chat_payload(kind: WebhookKind, text: &str) -> serde_json::Value {
    match kind {
        WebhookKind::Slack => serde_json::json!({ "text": truncate(text, 39_000) }),
        WebhookKind::Discord => serde_json::json!({ "content": truncate(text, 2000) }),
        WebhookKind::Generic => serde_json::json!({ "source": "skry", "text": text }),
    }
}

pub fn notification_payload(kind: WebhookKind, n: &Notification) -> serde_json::Value {
    match kind {
        WebhookKind::Generic => serde_json::to_value(GenericPayload {
            source: "skry",
            notification: n,
        })
        .unwrap_or_default(),
        chat => chat_payload(chat, &n.text()),
    }
}

/// Posts JSON to a webhook. Errors mention only the redacted URL.
pub async fn post(
    client: &reqwest::Client,
    hook: &Webhook,
    body: &serde_json::Value,
) -> Result<(), String> {
    let res = client
        .post(hook.url.expose())
        .timeout(Duration::from_secs(10))
        .json(body)
        .send()
        .await
        .map_err(|e| {
            format!(
                "webhook {} ({}): {}",
                hook.label(),
                redact_url(hook.url.expose()),
                e.without_url()
            )
        })?;
    if !res.status().is_success() {
        return Err(format!(
            "webhook {} ({}): HTTP {}",
            hook.label(),
            redact_url(hook.url.expose()),
            res.status()
        ));
    }
    Ok(())
}

/// Delivers notifications to every webhook subscribed to their category.
pub async fn deliver(client: &reqwest::Client, hooks: &[Webhook], notifications: &[Notification]) {
    for n in notifications {
        for hook in hooks.iter().filter(|h| h.events.contains(&n.category)) {
            let body = notification_payload(hook.kind, n);
            if let Err(e) = post(client, hook, &body).await {
                tracing::warn!("{e}");
            }
        }
    }
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("skry/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(name: &str, level: Level) -> Condition {
        Condition::new(
            "h",
            name,
            AlertCategory::Threshold,
            level,
            format!("{name} high"),
        )
    }

    fn kinds(n: &[Notification]) -> Vec<(NotificationKind, String)> {
        n.iter().map(|n| (n.kind, n.alert.clone())).collect()
    }

    const MIN: Duration = Duration::from_secs(60);

    #[test]
    fn fires_once_then_deduplicates() {
        let mut m = AlertManager::new(10 * MIN, true);
        let t = Instant::now();
        let n = m.evaluate("h", vec![cond("cpu", Level::Warning)], t);
        assert_eq!(kinds(&n), vec![(NotificationKind::Firing, "cpu".into())]);
        for i in 1..20 {
            let n = m.evaluate(
                "h",
                vec![cond("cpu", Level::Warning)],
                t + Duration::from_secs(i * 2),
            );
            assert!(n.is_empty(), "duplicate at tick {i}");
        }
    }

    #[test]
    fn reminder_after_cooldown() {
        let mut m = AlertManager::new(10 * MIN, true);
        let t = Instant::now();
        m.evaluate("h", vec![cond("cpu", Level::Warning)], t);
        let n = m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 10 * MIN);
        assert_eq!(kinds(&n), vec![(NotificationKind::Reminder, "cpu".into())]);
    }

    #[test]
    fn escalation_bypasses_cooldown() {
        let mut m = AlertManager::new(10 * MIN, true);
        let t = Instant::now();
        m.evaluate("h", vec![cond("cpu", Level::Warning)], t);
        let n = m.evaluate("h", vec![cond("cpu", Level::Critical)], t + MIN);
        assert_eq!(kinds(&n), vec![(NotificationKind::Escalated, "cpu".into())]);
        // De-escalation is silent.
        assert!(
            m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 2 * MIN)
                .is_empty()
        );
    }

    #[test]
    fn resolve_and_flapping_suppression() {
        let mut m = AlertManager::new(10 * MIN, true);
        let t = Instant::now();
        m.evaluate("h", vec![cond("cpu", Level::Warning)], t);
        let n = m.evaluate("h", vec![], t + MIN);
        assert_eq!(kinds(&n), vec![(NotificationKind::Resolved, "cpu".into())]);
        // Comes back within the cooldown: suppressed, and so is its resolution.
        assert!(
            m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 2 * MIN)
                .is_empty()
        );
        assert!(m.evaluate("h", vec![], t + 3 * MIN).is_empty());
        // After the cooldown it fires again.
        let n = m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 11 * MIN);
        assert_eq!(kinds(&n), vec![(NotificationKind::Firing, "cpu".into())]);
    }

    #[test]
    fn suppressed_condition_fires_when_cooldown_ends() {
        let mut m = AlertManager::new(10 * MIN, true);
        let t = Instant::now();
        m.evaluate("h", vec![cond("cpu", Level::Warning)], t);
        m.evaluate("h", vec![], t + MIN);
        assert!(
            m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 2 * MIN)
                .is_empty()
        );
        let n = m.evaluate("h", vec![cond("cpu", Level::Warning)], t + 10 * MIN);
        assert_eq!(kinds(&n), vec![(NotificationKind::Firing, "cpu".into())]);
    }

    #[test]
    fn hosts_and_keys_are_independent() {
        let mut m = AlertManager::new(10 * MIN, false);
        let t = Instant::now();
        let n = m.evaluate(
            "h",
            vec![cond("cpu", Level::Warning), cond("disk", Level::Critical)],
            t,
        );
        assert_eq!(n.len(), 2);
        let other = Condition::new("g", "cpu", AlertCategory::Threshold, Level::Warning, "x");
        assert_eq!(m.evaluate("g", vec![other], t).len(), 1);
        // Clearing h does not touch g, and resolved notifications are off.
        assert!(m.evaluate("h", vec![], t + MIN).is_empty());
        assert_eq!(m.active_count(), 1);
    }

    #[test]
    fn payload_shapes() {
        let n =
            Notification::from_condition(NotificationKind::Firing, &cond("cpu", Level::Critical));
        let slack = notification_payload(WebhookKind::Slack, &n);
        assert!(
            slack["text"]
                .as_str()
                .unwrap()
                .contains("[CRITICAL] h: cpu")
        );
        let discord = notification_payload(WebhookKind::Discord, &n);
        assert!(discord["content"].is_string());
        let generic = notification_payload(WebhookKind::Generic, &n);
        assert_eq!(generic["source"], "skry");
        assert_eq!(generic["kind"], "firing");
        assert_eq!(generic["level"], "critical");
        assert_eq!(generic["category"], "threshold");
        let long = "x".repeat(5000);
        assert_eq!(
            chat_payload(WebhookKind::Discord, &long)["content"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            2000
        );
    }

    #[tokio::test]
    async fn delivers_to_subscribed_webhooks_only() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut bodies = Vec::new();
            for _ in 0..1 {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap();
                bodies.push(String::from_utf8_lossy(&buf[..n]).to_string());
                s.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                    .await
                    .unwrap();
            }
            bodies
        });
        let hook = |events| Webhook {
            name: "t".into(),
            kind: WebhookKind::Generic,
            url: crate::config::Secret::new(format!("http://{addr}/hook")),
            events,
            snapshot: false,
        };
        let hooks = vec![
            hook(vec![AlertCategory::Threshold]),
            hook(vec![AlertCategory::Security]),
        ];
        let n =
            Notification::from_condition(NotificationKind::Firing, &cond("cpu", Level::Warning));
        deliver(&http_client(), &hooks, &[n]).await;
        let bodies = server.await.unwrap();
        assert_eq!(bodies.len(), 1);
        assert!(bodies[0].contains("\"alert\":\"cpu\""));
    }
}
