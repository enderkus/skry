//! Schedules collection for every host in parallel and maintains the
//! shared fleet state.
//!
//! Each host has its own task (so one slow or failing host never blocks the
//! others) that keeps one SSH session open, reconnects with exponential
//! backoff, and runs the batched collection command once per tick. A
//! semaphore bounds how many hosts are contacted at the same time. Results
//! flow to a single processor task that evaluates health, baselines and
//! alerts, writes history, and publishes the new [`FleetState`].

mod collector;
pub mod once;
mod processor;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::task::JoinHandle;

use crate::baseline::{Baselines, Deviation};
use crate::config::{AlertConfig, BaselineConfig, Config, SecurityConfig, Target, Webhook};
use crate::model::{Health, HostMetrics, Level, Thresholds};
use crate::security::{Finding, TlsResult};
use crate::ssh::{FailureKind, ResolvedHost, SshError, SshOptions};
use crate::store::{Store, StoreMsg};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum HostStatus {
    #[default]
    Pending,
    Ok,
    Deviation,
    Warning,
    Critical,
    Unreachable,
}

impl HostStatus {
    pub fn label(self) -> &'static str {
        match self {
            HostStatus::Pending => "pending",
            HostStatus::Ok => "ok",
            HostStatus::Deviation => "deviation",
            HostStatus::Warning => "warning",
            HostStatus::Critical => "critical",
            HostStatus::Unreachable => "unreachable",
        }
    }

    /// Severity stored in history; higher is worse.
    pub fn rank(self) -> i64 {
        match self {
            HostStatus::Pending | HostStatus::Ok => 0,
            HostStatus::Deviation => 1,
            HostStatus::Warning => 2,
            HostStatus::Critical => 3,
            HostStatus::Unreachable => 4,
        }
    }

    pub fn from_rank(rank: i64) -> Self {
        match rank {
            1 => HostStatus::Deviation,
            2 => HostStatus::Warning,
            3 => HostStatus::Critical,
            4 => HostStatus::Unreachable,
            _ => HostStatus::Ok,
        }
    }

    pub fn from_parts(connected: bool, has_metrics: bool, level: Level, deviating: bool) -> Self {
        if !connected {
            HostStatus::Unreachable
        } else if !has_metrics {
            HostStatus::Pending
        } else {
            match level {
                Level::Critical => HostStatus::Critical,
                Level::Warning => HostStatus::Warning,
                Level::Ok if deviating => HostStatus::Deviation,
                Level::Ok => HostStatus::Ok,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnState {
    #[default]
    Idle,
    Connecting,
    Connected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostError {
    pub kind: FailureKind,
    pub message: String,
    pub since: DateTime<Utc>,
}

impl HostError {
    pub fn from_ssh(e: &SshError, since: DateTime<Utc>) -> Self {
        HostError {
            kind: e.kind(),
            message: e.to_string(),
            since,
        }
    }
}

/// Everything known about one host right now.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HostState {
    pub name: String,
    pub addr: Option<String>,
    pub groups: Vec<String>,
    pub allowed_ports: Option<Vec<u16>>,
    pub status: HostStatus,
    pub conn: ConnState,
    pub error: Option<HostError>,
    pub metrics: Option<HostMetrics>,
    pub health: Health,
    pub deviations: Vec<Deviation>,
    pub findings: Vec<Finding>,
    pub last_update: Option<DateTime<Utc>>,
}

impl HostState {
    pub fn new(target: &Target, addr: Option<String>) -> Self {
        HostState {
            name: target.spec.clone(),
            addr,
            groups: target.groups.clone(),
            allowed_ports: target
                .allowed_ports
                .as_ref()
                .map(|p| p.iter().copied().collect()),
            ..Default::default()
        }
    }

    pub fn refresh_status(&mut self) {
        self.status = HostStatus::from_parts(
            self.conn != ConnState::Failed,
            self.metrics.is_some(),
            self.health.level,
            !self.deviations.is_empty(),
        );
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetState {
    pub hosts: Vec<HostState>,
    pub tls: Vec<TlsResult>,
    pub started: DateTime<Utc>,
    pub generation: u64,
}

impl FleetState {
    pub fn host(&self, name: &str) -> Option<&HostState> {
        self.hosts.iter().find(|h| h.name == name)
    }

    pub fn counts(&self) -> [(HostStatus, usize); 6] {
        let mut out = [
            (HostStatus::Ok, 0),
            (HostStatus::Deviation, 0),
            (HostStatus::Warning, 0),
            (HostStatus::Critical, 0),
            (HostStatus::Unreachable, 0),
            (HostStatus::Pending, 0),
        ];
        for h in &self.hosts {
            if let Some(slot) = out.iter_mut().find(|(s, _)| *s == h.status) {
                slot.1 += 1;
            }
        }
        out
    }
}

/// A host to monitor: the command-line target and its resolved connection
/// parameters (or why resolution failed).
#[derive(Debug, Clone)]
pub struct HostPlan {
    pub target: Target,
    pub resolved: Result<ResolvedHost, String>,
}

/// Engine settings derived from the configuration.
#[derive(Debug, Clone)]
pub struct Settings {
    pub interval: Duration,
    pub slow_interval: Duration,
    pub command_timeout: Duration,
    pub concurrency: usize,
    pub thresholds: Thresholds,
    pub security: SecurityConfig,
    pub baseline: BaselineConfig,
    pub alerts: AlertConfig,
    pub webhooks: Vec<Webhook>,
    pub history_path: Option<PathBuf>,
    pub retention_hours: u64,
    pub detail_interval: Duration,
    pub tls_endpoints: Vec<String>,
}

impl Settings {
    pub fn from_config(cfg: &Config, tls_endpoints: Vec<String>) -> Self {
        Settings {
            interval: Duration::from_secs_f64(cfg.interval),
            slow_interval: Duration::from_secs_f64(cfg.slow_interval.max(cfg.interval)),
            command_timeout: Duration::from_secs_f64(cfg.command_timeout),
            concurrency: cfg.concurrency,
            thresholds: cfg.thresholds,
            security: cfg.security.clone(),
            baseline: cfg.baseline.clone(),
            alerts: cfg.alerts.clone(),
            webhooks: cfg.webhooks.clone(),
            history_path: cfg.history.enabled.then(|| cfg.history_path()).flatten(),
            retention_hours: cfg.history.retention_hours,
            detail_interval: Duration::from_secs(cfg.history.detail_interval.max(1)),
            tls_endpoints,
        }
    }
}

/// Messages from host collectors to the processor.
#[derive(Debug)]
pub(crate) enum Event {
    Connecting(usize),
    Sample(usize, Box<HostMetrics>),
    Failed(usize, HostError),
    Tls(Vec<TlsResult>),
}

/// A running engine.
pub struct Engine {
    fleet: Arc<RwLock<FleetState>>,
    changes: watch::Receiver<u64>,
    shutdown: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    processor: Option<JoinHandle<()>>,
    store_thread: Option<std::thread::JoinHandle<()>>,
    history_path: Option<PathBuf>,
    settings: Settings,
}

impl Engine {
    pub fn start(plans: Vec<HostPlan>, settings: Settings, ssh: SshOptions) -> Engine {
        let hosts = plans
            .iter()
            .map(|p| {
                HostState::new(
                    &p.target,
                    p.resolved.as_ref().ok().map(|r| r.display_addr()),
                )
            })
            .collect();
        let fleet = Arc::new(RwLock::new(FleetState {
            hosts,
            tls: Vec::new(),
            started: Utc::now(),
            generation: 0,
        }));
        let (change_tx, changes) = watch::channel(0u64);
        let (shutdown, shutdown_rx) = watch::channel(false);
        let (event_tx, event_rx) = mpsc::channel::<Event>(1024);

        let mut baselines = Baselines::new(settings.baseline.clone());
        let mut store_tx = None;
        let mut store_thread = None;
        let mut history_path = None;
        if let Some(path) = &settings.history_path {
            match Store::open(path) {
                Ok(store) => {
                    if let Ok(rows) = store.load_baselines() {
                        baselines.import(&rows);
                    }
                    let (tx, handle) = crate::store::spawn_writer(store, settings.retention_hours);
                    store_tx = Some(tx);
                    store_thread = Some(handle);
                    history_path = Some(path.clone());
                }
                Err(e) => tracing::warn!(error = %e, "history disabled"),
            }
        }

        let processor = processor::Processor::new(
            fleet.clone(),
            change_tx,
            settings.clone(),
            baselines,
            store_tx,
        );
        let processor = tokio::spawn(processor.run(event_rx, shutdown_rx.clone()));

        let semaphore = Arc::new(Semaphore::new(settings.concurrency.max(1)));
        let mut tasks = Vec::new();
        for (idx, plan) in plans.into_iter().enumerate() {
            let ctx = collector::Collector {
                idx,
                plan,
                ssh: ssh.clone(),
                settings: settings.clone(),
                semaphore: semaphore.clone(),
                events: event_tx.clone(),
                shutdown: shutdown_rx.clone(),
            };
            tasks.push(tokio::spawn(ctx.run()));
        }
        if !settings.tls_endpoints.is_empty() {
            tasks.push(tokio::spawn(collector::tls_loop(
                settings.clone(),
                event_tx.clone(),
                shutdown_rx.clone(),
            )));
        }

        Engine {
            fleet,
            changes,
            shutdown,
            tasks,
            processor: Some(processor),
            store_thread,
            history_path,
            settings,
        }
    }

    /// A consistent copy of the current fleet state.
    pub fn snapshot(&self) -> FleetState {
        self.fleet.read().expect("fleet lock").clone()
    }

    pub fn fleet(&self) -> Arc<RwLock<FleetState>> {
        self.fleet.clone()
    }

    /// Receiver that changes whenever the fleet state is updated.
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.changes.clone()
    }

    pub fn history_path(&self) -> Option<&PathBuf> {
        self.history_path.as_ref()
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Stops all tasks, persists baselines and flushes history.
    pub async fn shutdown(mut self) {
        let _ = self.shutdown.send(true);
        for t in self.tasks.drain(..) {
            let _ = tokio::time::timeout(Duration::from_secs(3), t).await;
        }
        if let Some(p) = self.processor.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), p).await;
        }
        if let Some(t) = self.store_thread.take() {
            let _ = tokio::task::spawn_blocking(move || t.join()).await;
        }
    }
}

/// Sends a message to the history writer, ignoring a stopped writer.
pub(crate) fn store_send(tx: &Option<std::sync::mpsc::Sender<StoreMsg>>, msg: StoreMsg) {
    if let Some(tx) = tx {
        let _ = tx.send(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_precedence() {
        use HostStatus::*;
        assert_eq!(
            HostStatus::from_parts(false, true, Level::Ok, false),
            Unreachable
        );
        assert_eq!(
            HostStatus::from_parts(true, false, Level::Ok, false),
            Pending
        );
        assert_eq!(
            HostStatus::from_parts(true, true, Level::Critical, true),
            Critical
        );
        assert_eq!(
            HostStatus::from_parts(true, true, Level::Warning, true),
            Warning
        );
        assert_eq!(
            HostStatus::from_parts(true, true, Level::Ok, true),
            Deviation
        );
        assert_eq!(HostStatus::from_parts(true, true, Level::Ok, false), Ok);
        for s in [Ok, Deviation, Warning, Critical, Unreachable] {
            assert_eq!(HostStatus::from_rank(s.rank()), s);
        }
    }
}
