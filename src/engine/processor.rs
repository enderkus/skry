//! Central processing of collection results: health, baselines, security
//! findings, alerts and history.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::{Timelike, Utc};
use tokio::sync::{mpsc, watch};

use super::{ConnState, Event, FleetState, HostState, Settings, store_send};
use crate::alert::{AlertManager, Condition, Notification};
use crate::baseline::Baselines;
use crate::config::AlertCategory;
use crate::model::{Health, Level};
use crate::security::host_findings;
use crate::store::{SampleRow, StoreMsg};

pub(crate) struct Processor {
    fleet: Arc<RwLock<FleetState>>,
    changes: watch::Sender<u64>,
    settings: Settings,
    baselines: Baselines,
    alerts: AlertManager,
    store: Option<std::sync::mpsc::Sender<StoreMsg>>,
    last_detail: HashMap<usize, Instant>,
    last_failure_row: HashMap<usize, Instant>,
    /// Last threshold/baseline/security conditions per host, kept while the
    /// host is unreachable so they are not reported as resolved.
    last_conditions: HashMap<usize, Vec<Condition>>,
    notify: Option<mpsc::UnboundedSender<Vec<Notification>>>,
}

/// Alert conditions derived from a reachable host's state.
pub(crate) fn conditions_for(h: &HostState) -> Vec<Condition> {
    let mut out = Vec::new();
    for b in &h.health.breaches {
        out.push(Condition::new(
            &h.name,
            format!("threshold:{}", b.metric),
            AlertCategory::Threshold,
            b.level,
            b.describe(),
        ));
    }
    for d in &h.deviations {
        out.push(Condition::new(
            &h.name,
            format!("baseline:{}", d.metric.name()),
            AlertCategory::Baseline,
            Level::Warning,
            d.describe(),
        ));
    }
    for f in &h.findings {
        out.push(Condition::new(
            &h.name,
            format!("security:{}", f.name),
            AlertCategory::Security,
            f.level,
            f.message.clone(),
        ));
    }
    out
}

impl Processor {
    pub fn new(
        fleet: Arc<RwLock<FleetState>>,
        changes: watch::Sender<u64>,
        settings: Settings,
        baselines: Baselines,
        store: Option<std::sync::mpsc::Sender<StoreMsg>>,
    ) -> Self {
        let notify = (!settings.webhooks.is_empty()).then(|| {
            let (tx, mut rx) = mpsc::unbounded_channel::<Vec<Notification>>();
            let hooks = settings.webhooks.clone();
            tokio::spawn(async move {
                let client = crate::alert::http_client();
                while let Some(batch) = rx.recv().await {
                    crate::alert::deliver(&client, &hooks, &batch).await;
                }
            });
            tx
        });
        Processor {
            alerts: AlertManager::new(
                Duration::from_secs(settings.alerts.cooldown),
                settings.alerts.notify_resolved,
            ),
            fleet,
            changes,
            settings,
            baselines,
            store,
            last_detail: HashMap::new(),
            last_failure_row: HashMap::new(),
            last_conditions: HashMap::new(),
            notify,
        }
    }

    pub async fn run(
        mut self,
        mut events: mpsc::Receiver<Event>,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let mut periodic = tokio::time::interval(Duration::from_secs(15));
        let mut last_baseline_save = Instant::now();
        loop {
            tokio::select! {
                ev = events.recv() => match ev {
                    Some(ev) => self.handle(ev),
                    None => break,
                },
                _ = periodic.tick() => {
                    self.check_unreachable();
                    if last_baseline_save.elapsed() > Duration::from_secs(300) {
                        last_baseline_save = Instant::now();
                        store_send(&self.store, StoreMsg::Baselines(self.baselines.export()));
                    }
                }
                _ = shutdown.changed() => break,
            }
        }
        store_send(&self.store, StoreMsg::Baselines(self.baselines.export()));
    }

    fn publish(&self, fleet: &mut FleetState) {
        fleet.generation += 1;
        let _ = self.changes.send(fleet.generation);
    }

    fn notify(&self, n: Vec<Notification>) {
        if n.is_empty() {
            return;
        }
        for x in &n {
            tracing::info!(host = %x.host, alert = %x.alert, kind = ?x.kind, "{}", x.message);
        }
        if let Some(tx) = &self.notify {
            let _ = tx.send(n);
        }
    }

    fn handle(&mut self, ev: Event) {
        let fleet_lock = self.fleet.clone();
        let mut fleet = fleet_lock.write().expect("fleet lock");
        match ev {
            Event::Connecting(idx) => {
                if let Some(h) = fleet.hosts.get_mut(idx)
                    && h.conn != ConnState::Failed
                {
                    h.conn = ConnState::Connecting;
                    h.refresh_status();
                }
            }
            Event::Sample(idx, metrics) => {
                let Some(h) = fleet.hosts.get_mut(idx) else {
                    return;
                };
                let now = Utc::now();
                h.conn = ConnState::Connected;
                h.error = None;
                h.health = Health::evaluate(&metrics, &self.settings.thresholds);
                h.deviations =
                    self.baselines
                        .observe(&h.name, &metrics, chrono::Local::now().hour());
                let allowed: Option<BTreeSet<u16>> = h
                    .allowed_ports
                    .as_ref()
                    .map(|p| p.iter().copied().collect());
                h.findings = host_findings(&metrics, allowed.as_ref(), &self.settings.security);
                h.metrics = Some(*metrics);
                h.last_update = Some(now);
                h.refresh_status();

                let conditions = conditions_for(h);
                self.last_conditions.insert(idx, conditions.clone());
                let notes = self.alerts.evaluate(&h.name, conditions, Instant::now());

                let m = h.metrics.as_ref().expect("just set");
                store_send(
                    &self.store,
                    StoreMsg::Samples(vec![SampleRow {
                        host: h.name.clone(),
                        ts: now.timestamp_millis(),
                        res: 0,
                        status: h.status.rank(),
                        cpu: m.cpu_pct(),
                        mem: m.mem_pct(),
                        disk: m.disk_max_pct(),
                        load: m.load_per_core,
                        rx: m.net_rx_bps,
                        tx: m.net_tx_bps,
                    }]),
                );
                let detail_due = self
                    .last_detail
                    .get(&idx)
                    .is_none_or(|t| t.elapsed() >= self.settings.detail_interval);
                if detail_due && self.store.is_some() {
                    self.last_detail.insert(idx, Instant::now());
                    if let Ok(json) = serde_json::to_string(&*h) {
                        store_send(
                            &self.store,
                            StoreMsg::Detail {
                                host: h.name.clone(),
                                ts: now.timestamp_millis(),
                                json,
                            },
                        );
                    }
                }
                self.publish(&mut fleet);
                drop(fleet);
                self.notify(notes);
            }
            Event::Failed(idx, err) => {
                let Some(h) = fleet.hosts.get_mut(idx) else {
                    return;
                };
                let since = h
                    .error
                    .as_ref()
                    .filter(|_| h.conn == ConnState::Failed)
                    .map(|e| e.since)
                    .unwrap_or(err.since);
                h.error = Some(super::HostError { since, ..err });
                h.conn = ConnState::Failed;
                h.refresh_status();
                let row_due = self
                    .last_failure_row
                    .get(&idx)
                    .is_none_or(|t| t.elapsed() >= self.settings.interval);
                if row_due {
                    self.last_failure_row.insert(idx, Instant::now());
                    store_send(
                        &self.store,
                        StoreMsg::Samples(vec![SampleRow {
                            host: h.name.clone(),
                            ts: Utc::now().timestamp_millis(),
                            res: 0,
                            status: h.status.rank(),
                            cpu: None,
                            mem: None,
                            disk: None,
                            load: None,
                            rx: None,
                            tx: None,
                        }]),
                    );
                }
                self.publish(&mut fleet);
                drop(fleet);
                self.check_unreachable();
            }
            Event::Tls(results) => {
                let mut notes = Vec::new();
                for r in &results {
                    let host = format!("tls:{}", r.endpoint);
                    let conds = if r.level == Level::Ok {
                        Vec::new()
                    } else {
                        vec![Condition::new(
                            &host,
                            "tls_certificate",
                            AlertCategory::Security,
                            r.level,
                            r.describe(),
                        )]
                    };
                    notes.extend(self.alerts.evaluate(&host, conds, Instant::now()));
                }
                fleet.tls = results;
                self.publish(&mut fleet);
                drop(fleet);
                self.notify(notes);
            }
        }
    }

    /// Raises unreachable alerts for hosts that have been failing longer
    /// than `unreachable_after`.
    fn check_unreachable(&mut self) {
        let after = chrono::Duration::seconds(self.settings.alerts.unreachable_after as i64);
        let now = Utc::now();
        let failing: Vec<(usize, String, String, chrono::DateTime<Utc>)> = {
            let fleet = self.fleet.read().expect("fleet lock");
            fleet
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, h)| h.conn == ConnState::Failed)
                .filter_map(|(i, h)| {
                    h.error
                        .as_ref()
                        .map(|e| (i, h.name.clone(), e.message.clone(), e.since))
                })
                .collect()
        };
        let mut notes = Vec::new();
        for (idx, name, message, since) in failing {
            let mut conds = self.last_conditions.get(&idx).cloned().unwrap_or_default();
            if now - since >= after {
                conds.push(Condition::new(
                    &name,
                    "unreachable",
                    AlertCategory::Unreachable,
                    Level::Critical,
                    message,
                ));
            }
            notes.extend(self.alerts.evaluate(&name, conds, Instant::now()));
        }
        self.notify(notes);
    }
}
