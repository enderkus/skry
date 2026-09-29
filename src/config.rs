//! Configuration file loading, host groups and defaults.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::Thresholds;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("unknown host group @{0}")]
    UnknownGroup(String),
    #[error("host group @{0} includes itself")]
    GroupCycle(String),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// A string that never appears in logs or debug output.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: impl Into<String>) -> Self {
        Secret(s.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"<redacted>\"")
    }
}

/// Reduces a URL to scheme and host so it can be logged safely.
pub fn redact_url(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest.split(['/', '?', '#']).next().unwrap_or("");
            let host = host.rsplit('@').next().unwrap_or(host);
            format!("{scheme}://{host}/<redacted>")
        }
        None => "<redacted>".into(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Seconds between collection ticks.
    pub interval: f64,
    /// Seconds between collections of slow sections (ports, containers,
    /// services, logins, updates).
    pub slow_interval: f64,
    /// Maximum number of hosts contacted at the same time.
    pub concurrency: usize,
    /// Seconds allowed for TCP connect, handshake and authentication.
    pub connect_timeout: f64,
    /// Seconds allowed for one remote collection command.
    pub command_timeout: f64,
    pub thresholds: Thresholds,
    pub history: HistoryConfig,
    pub baseline: BaselineConfig,
    pub alerts: AlertConfig,
    pub webhooks: Vec<Webhook>,
    pub security: SecurityConfig,
    pub web: WebConfig,
    pub snapshot: SnapshotConfig,
    pub groups: BTreeMap<String, Group>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 2.0,
            slow_interval: 60.0,
            concurrency: 32,
            connect_timeout: 10.0,
            command_timeout: 30.0,
            thresholds: Thresholds::default(),
            history: HistoryConfig::default(),
            baseline: BaselineConfig::default(),
            alerts: AlertConfig::default(),
            webhooks: Vec::new(),
            security: SecurityConfig::default(),
            web: WebConfig::default(),
            snapshot: SnapshotConfig::default(),
            groups: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub enabled: bool,
    /// Hours of history to keep.
    pub retention_hours: u64,
    /// Database path; defaults to the platform data directory.
    pub path: Option<PathBuf>,
    /// Seconds between full host detail records (summaries are stored on
    /// every tick).
    pub detail_interval: u64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_hours: 24,
            path: None,
            detail_interval: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BaselineConfig {
    pub enabled: bool,
    /// EWMA smoothing factor per sample (0 < alpha < 1).
    pub alpha: f64,
    /// Flag values whose z-score exceeds this.
    pub z_threshold: f64,
    /// Samples required before a baseline is trusted.
    pub warmup: u64,
    /// Keep a separate baseline for each hour of the day.
    pub hourly: bool,
}

impl Default for BaselineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            alpha: 0.02,
            z_threshold: 3.5,
            warmup: 300,
            hourly: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AlertConfig {
    /// Minimum seconds between two notifications for the same alert.
    pub cooldown: u64,
    /// Send a notification when an alert clears.
    pub notify_resolved: bool,
    /// Seconds a host must be unreachable before alerting.
    pub unreachable_after: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            cooldown: 900,
            notify_resolved: true,
            unreachable_after: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookKind {
    Slack,
    Discord,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertCategory {
    Threshold,
    Baseline,
    Unreachable,
    Security,
}

impl AlertCategory {
    pub const ALL: [AlertCategory; 4] = [
        AlertCategory::Threshold,
        AlertCategory::Baseline,
        AlertCategory::Unreachable,
        AlertCategory::Security,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AlertCategory::Threshold => "threshold",
            AlertCategory::Baseline => "baseline",
            AlertCategory::Unreachable => "unreachable",
            AlertCategory::Security => "security",
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    #[serde(default)]
    pub name: String,
    pub kind: WebhookKind,
    pub url: Secret,
    /// Alert categories delivered to this webhook; all when omitted.
    #[serde(default = "all_categories")]
    pub events: Vec<AlertCategory>,
    /// Also post incident snapshots here.
    #[serde(default)]
    pub snapshot: bool,
}

fn all_categories() -> Vec<AlertCategory> {
    AlertCategory::ALL.to_vec()
}

impl fmt::Debug for Webhook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Webhook")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("url", &redact_url(self.url.expose()))
            .field("events", &self.events)
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

impl Webhook {
    pub fn label(&self) -> String {
        if self.name.is_empty() {
            format!("{:?}", self.kind).to_lowercase()
        } else {
            self.name.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SecurityConfig {
    /// Failed SSH authentication events in 24 hours that count as a finding.
    pub failed_login_threshold: u64,
    /// Query dnf's cache for pending updates on RPM systems. Off by default
    /// because dnf appends to its own log files even for read-only queries,
    /// and it requires root.
    pub rpm_updates: bool,
    /// TLS endpoints (`host:port`) checked from the local machine for every run.
    pub tls: Vec<String>,
    pub tls_warning_days: i64,
    pub tls_critical_days: i64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            failed_login_threshold: 50,
            rpm_updates: false,
            tls: Vec::new(),
            tls_warning_days: 21,
            tls_critical_days: 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebConfig {
    pub bind: String,
    /// Required when binding to anything other than a loopback address.
    /// The `SKRY_WEB_TOKEN` environment variable takes precedence.
    pub token: Option<Secret>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:9187".into(),
            token: None,
        }
    }
}

impl WebConfig {
    pub fn effective_token(&self) -> Option<Secret> {
        std::env::var("SKRY_WEB_TOKEN")
            .ok()
            .filter(|t| !t.is_empty())
            .map(Secret::new)
            .or_else(|| self.token.clone().filter(|t| !t.is_empty()))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SnapshotConfig {
    /// Directory for snapshot files; the current directory by default.
    pub dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Group {
    /// Host names from `~/.ssh/config`, `user@host:port` or `@othergroup`.
    pub hosts: Vec<String>,
    /// Ports expected to listen; others are highlighted. Empty = no policy.
    pub allowed_ports: Vec<u16>,
    /// TLS endpoints checked when this group is monitored.
    pub tls: Vec<String>,
}

/// A host selected on the command line, with the policy of the group it
/// came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub spec: String,
    pub groups: Vec<String>,
    pub allowed_ports: Option<BTreeSet<u16>>,
}

impl Config {
    /// Default config file location. `~/.config/skry/config.toml` is
    /// honoured on every platform when present, then the platform config
    /// directory (e.g. `~/Library/Application Support/skry` on macOS).
    pub fn default_path() -> Option<PathBuf> {
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| directories::BaseDirs::new().map(|b| b.home_dir().join(".config")))
            .map(|d| d.join("skry").join("config.toml"));
        if let Some(p) = &xdg
            && p.exists()
        {
            return xdg;
        }
        directories::ProjectDirs::from("", "", "skry")
            .map(|d| d.config_dir().join("config.toml"))
            .or(xdg)
    }

    pub fn default_data_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "skry").map(|d| d.data_dir().to_path_buf())
    }

    /// Loads the config at `path`, or the default path. A missing default
    /// file is not an error; a missing explicit file is.
    pub fn load(path: Option<&Path>) -> Result<Config, ConfigError> {
        let (path, explicit) = match path {
            Some(p) => (p.to_path_buf(), true),
            None => match Self::default_path() {
                Some(p) => (p, false),
                None => return Ok(Config::default()),
            },
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text, &path),
            Err(e) if !explicit && e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(source) => Err(ConfigError::Read { path, source }),
        }
    }

    pub fn parse(text: &str, path: &Path) -> Result<Config, ConfigError> {
        let cfg: Config = toml::from_str(text).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(e),
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |m: &str| Err(ConfigError::Invalid(m.to_string()));
        if self.interval.is_nan() || self.interval < 0.2 {
            return invalid("interval must be at least 0.2 seconds");
        }
        if self.concurrency == 0 {
            return invalid("concurrency must be at least 1");
        }
        if !(self.baseline.alpha > 0.0 && self.baseline.alpha < 1.0) {
            return invalid("baseline.alpha must be between 0 and 1");
        }
        if self.baseline.z_threshold <= 0.0 {
            return invalid("baseline.z_threshold must be positive");
        }
        for w in &self.webhooks {
            let url = w.url.expose();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(ConfigError::Invalid(format!(
                    "webhook {} must use an http(s) URL",
                    w.label()
                )));
            }
        }
        for (name, t) in [
            ("cpu", self.thresholds.cpu),
            ("memory", self.thresholds.memory),
            ("disk", self.thresholds.disk),
            ("load_per_core", self.thresholds.load_per_core),
        ] {
            if t.warning > t.critical {
                return Err(ConfigError::Invalid(format!(
                    "thresholds.{name}: warning is above critical"
                )));
            }
        }
        Ok(())
    }

    pub fn history_path(&self) -> Option<PathBuf> {
        self.history
            .path
            .clone()
            .or_else(|| Self::default_data_dir().map(|d| d.join("history.db")))
    }

    /// Expands command-line targets (`@group`, host names, `user@host:port`)
    /// into a de-duplicated host list.
    pub fn resolve_targets(&self, args: &[String]) -> Result<Vec<Target>, ConfigError> {
        let mut out: Vec<Target> = Vec::new();
        for arg in args {
            if let Some(group) = arg.strip_prefix('@') {
                self.expand_group(group, &mut Vec::new(), &mut out)?;
            } else {
                push_target(&mut out, arg, None, None);
            }
        }
        Ok(out)
    }

    fn expand_group(
        &self,
        name: &str,
        stack: &mut Vec<String>,
        out: &mut Vec<Target>,
    ) -> Result<(), ConfigError> {
        if stack.iter().any(|s| s == name) {
            return Err(ConfigError::GroupCycle(name.to_string()));
        }
        let group = self
            .groups
            .get(name)
            .ok_or_else(|| ConfigError::UnknownGroup(name.to_string()))?;
        stack.push(name.to_string());
        for host in &group.hosts {
            if let Some(inner) = host.strip_prefix('@') {
                self.expand_group(inner, stack, out)?;
            } else {
                let ports = (!group.allowed_ports.is_empty())
                    .then(|| group.allowed_ports.iter().copied().collect());
                push_target(out, host, Some(name), ports);
            }
        }
        stack.pop();
        Ok(())
    }

    /// TLS endpoints relevant to the given targets: global ones plus those
    /// of every group involved.
    pub fn tls_endpoints(&self, targets: &[Target]) -> Vec<String> {
        let mut set: BTreeSet<String> = self.security.tls.iter().cloned().collect();
        for t in targets {
            for g in &t.groups {
                if let Some(group) = self.groups.get(g) {
                    set.extend(group.tls.iter().cloned());
                }
            }
        }
        set.into_iter().collect()
    }
}

fn push_target(
    out: &mut Vec<Target>,
    spec: &str,
    group: Option<&str>,
    ports: Option<BTreeSet<u16>>,
) {
    if let Some(existing) = out.iter_mut().find(|t| t.spec == spec) {
        if let Some(g) = group
            && !existing.groups.iter().any(|x| x == g)
        {
            existing.groups.push(g.to_string());
        }
        // A host in several groups may listen on the union of their ports.
        match (&mut existing.allowed_ports, ports) {
            (Some(a), Some(b)) => a.extend(b),
            (slot @ None, Some(b)) if existing.groups.len() <= 1 => *slot = Some(b),
            _ => {}
        }
        return;
    }
    out.push(Target {
        spec: spec.to_string(),
        groups: group.map(|g| vec![g.to_string()]).unwrap_or_default(),
        allowed_ports: ports,
    });
}

/// A commented example configuration, printed by `skry config example`.
pub const EXAMPLE: &str = include_str!("../docs/config.example.toml");

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(text: &str) -> Config {
        Config::parse(text, Path::new("test.toml")).unwrap()
    }

    #[test]
    fn defaults() {
        let c = cfg("");
        assert_eq!(c.interval, 2.0);
        assert_eq!(c.history.retention_hours, 24);
        assert_eq!(c.web.bind, "127.0.0.1:9187");
        assert!(c.baseline.enabled);
    }

    #[test]
    fn example_config_is_valid() {
        let c = cfg(EXAMPLE);
        assert!(c.groups.contains_key("production"));
        assert!(!c.webhooks.is_empty());
    }

    #[test]
    fn groups_expand_and_dedupe() {
        let c = cfg(r#"
            [groups.web]
            hosts = ["web1", "web2"]
            allowed_ports = [22, 443]
            [groups.db]
            hosts = ["db1", "web1"]
            allowed_ports = [5432]
            [groups.production]
            hosts = ["@web", "@db"]
        "#);
        let t = c
            .resolve_targets(&["@production".into(), "extra".into()])
            .unwrap();
        let specs: Vec<&str> = t.iter().map(|t| t.spec.as_str()).collect();
        assert_eq!(specs, vec!["web1", "web2", "db1", "extra"]);
        let web1 = &t[0];
        assert_eq!(web1.groups, vec!["web", "db"]);
        assert_eq!(
            web1.allowed_ports
                .as_ref()
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![22, 443, 5432]
        );
        assert_eq!(t[3].allowed_ports, None);
    }

    #[test]
    fn group_errors() {
        let c = cfg("[groups.a]\nhosts = [\"@b\"]\n[groups.b]\nhosts = [\"@a\"]\n");
        assert!(matches!(
            c.resolve_targets(&["@a".into()]),
            Err(ConfigError::GroupCycle(_))
        ));
        assert!(matches!(
            c.resolve_targets(&["@nope".into()]),
            Err(ConfigError::UnknownGroup(_))
        ));
    }

    #[test]
    fn rejects_bad_values() {
        assert!(Config::parse("interval = 0", Path::new("x")).is_err());
        assert!(Config::parse("unknown_key = 1", Path::new("x")).is_err());
        assert!(
            Config::parse(
                "[thresholds.cpu]\nwarning = 90\ncritical = 80\n",
                Path::new("x")
            )
            .is_err()
        );
        assert!(
            Config::parse(
                "[[webhooks]]\nkind = \"slack\"\nurl = \"file:///etc/passwd\"\n",
                Path::new("x")
            )
            .is_err()
        );
    }

    #[test]
    fn secrets_are_redacted() {
        let c = cfg(r#"
            [web]
            token = "hunter2"
            [[webhooks]]
            kind = "slack"
            url = "https://hooks.slack.com/services/T000/B000/XXXXSECRET"
        "#);
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("hunter2"));
        assert!(!dbg.contains("XXXXSECRET"));
        assert!(dbg.contains("https://hooks.slack.com/<redacted>"));
    }

    #[test]
    fn redact_url_strips_credentials() {
        assert_eq!(
            redact_url("https://user:pw@example.com/path?x=1"),
            "https://example.com/<redacted>"
        );
    }

    #[test]
    fn tls_endpoints_merge() {
        let c = cfg(r#"
            [security]
            tls = ["a.example:443"]
            [groups.g]
            hosts = ["h"]
            tls = ["b.example:443"]
        "#);
        let t = c.resolve_targets(&["@g".into()]).unwrap();
        assert_eq!(c.tls_endpoints(&t), vec!["a.example:443", "b.example:443"]);
    }
}
