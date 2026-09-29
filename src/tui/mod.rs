//! Terminal UI: fleet grid, host details, security pulse and timeline.

pub mod app;
pub mod demo;
#[cfg(test)]
mod render_tests;
pub mod ui;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use ratatui::crossterm::event::{Event, EventStream, KeyEventKind};

use crate::collect::LoadAvg;
use crate::config::Config;
use crate::engine::{ConnState, Engine, FleetState, HostError, HostState, HostStatus};
use crate::model::{CpuUsage, DiskUsage, HostMetrics, MemUsage};
use crate::snapshot::Snapshot;
use crate::ssh::FailureKind;
use crate::store::{SampleRow, Store};
use app::{Action, App, Screen, TimeCursor};
use ui::{HistoryView, ViewData};

/// How far back a stored sample may be to represent a moment.
const HISTORY_WINDOW_MS: i64 = 15 * 60 * 1000;

/// Rebuilds a tile-level host view from a stored summary row.
fn host_from_row(base: &HostState, row: &SampleRow) -> HostState {
    let status = HostStatus::from_rank(row.status);
    let ts = DateTime::from_timestamp_millis(row.ts).unwrap_or_default();
    let mut h = HostState {
        name: base.name.clone(),
        addr: base.addr.clone(),
        groups: base.groups.clone(),
        allowed_ports: base.allowed_ports.clone(),
        status,
        conn: ConnState::Connected,
        last_update: Some(ts),
        ..Default::default()
    };
    if status == HostStatus::Unreachable {
        h.conn = ConnState::Failed;
        h.error = Some(HostError {
            kind: FailureKind::Network,
            message: "unreachable at this time".into(),
            since: ts,
        });
        return h;
    }
    h.metrics = Some(HostMetrics {
        ts,
        cpu: row.cpu.map(|c| CpuUsage {
            total_pct: c,
            ..Default::default()
        }),
        mem: row.mem.map(|m| MemUsage {
            used_pct: m,
            ..Default::default()
        }),
        disks: row
            .disk
            .map(|d| {
                vec![DiskUsage {
                    mount: "(highest)".into(),
                    used_pct: d,
                    ..Default::default()
                }]
            })
            .unwrap_or_default(),
        load_per_core: row.load,
        load: row.load.map(|l| LoadAvg {
            one: l,
            ..Default::default()
        }),
        net_rx_bps: row.rx,
        net_tx_bps: row.tx,
        ..Default::default()
    });
    h
}

/// The fleet as it was at `at`, from history. The selected host gets its
/// full stored record when one exists.
pub fn historical_fleet(
    store: &Store,
    at: DateTime<Utc>,
    live: &FleetState,
    selected: Option<&str>,
) -> (FleetState, Option<String>) {
    let at_ms = at.timestamp_millis();
    let rows = store.fleet_at(at_ms, HISTORY_WINDOW_MS).unwrap_or_default();
    let mut note = None;
    let hosts = live
        .hosts
        .iter()
        .map(|base| {
            if Some(base.name.as_str()) == selected
                && let Ok(Some((ts, json))) = store.detail_at(&base.name, at_ms, HISTORY_WINDOW_MS)
                && let Ok(mut h) = serde_json::from_str::<HostState>(&json)
            {
                let when = DateTime::from_timestamp_millis(ts).unwrap_or_default();
                note = Some(format!("details recorded {}", when.format("%H:%M:%S")));
                h.name = base.name.clone();
                return h;
            }
            match rows.iter().find(|r| r.host == base.name) {
                Some(row) => host_from_row(base, row),
                None => HostState {
                    name: base.name.clone(),
                    addr: base.addr.clone(),
                    groups: base.groups.clone(),
                    ..Default::default()
                },
            }
        })
        .collect();
    (
        FleetState {
            hosts,
            tls: live.tls.clone(),
            started: live.started,
            generation: live.generation,
        },
        note,
    )
}

fn expand_home(p: &std::path::Path) -> PathBuf {
    match (p.strip_prefix("~"), directories::BaseDirs::new()) {
        (Ok(rest), Some(b)) => b.home_dir().join(rest),
        _ => p.to_path_buf(),
    }
}

/// Writes a snapshot of what is on screen and returns a status message.
fn take_snapshot(app: &App, fleet: &FleetState, cfg: &Config) -> String {
    let names: Vec<String> = match app.screen {
        Screen::Host => app.selected.iter().cloned().collect(),
        _ => app
            .visible(&fleet.hosts)
            .into_iter()
            .map(|i| fleet.hosts[i].name.clone())
            .collect(),
    };
    let snap = Snapshot::from_fleet(fleet, Some(&names));
    let dir = cfg
        .snapshot
        .dir
        .as_deref()
        .map(expand_home)
        .unwrap_or_else(|| PathBuf::from("."));
    match snap.write(&dir) {
        Ok((md, _json)) => {
            let hooks: Vec<_> = cfg
                .webhooks
                .iter()
                .filter(|w| w.snapshot)
                .cloned()
                .collect();
            let posted = if hooks.is_empty() {
                String::new()
            } else {
                let s = snap.clone();
                tokio::spawn(async move {
                    for e in crate::snapshot::post(&s, &hooks).await {
                        tracing::warn!("{e}");
                    }
                });
                " and posted to webhooks".into()
            };
            format!(
                "snapshot of {} host(s) saved to {}{posted}",
                snap.hosts.len(),
                md.display()
            )
        }
        Err(e) => format!("snapshot failed: {e}"),
    }
}

fn history_view(store: Option<&Store>, host: Option<&str>, end: DateTime<Utc>) -> HistoryView {
    let Some(store) = store else {
        return HistoryView::default();
    };
    let start = store
        .time_range()
        .ok()
        .flatten()
        .and_then(|(a, _)| DateTime::from_timestamp_millis(a));
    let (cpu, mem) = host
        .and_then(|h| {
            let end_ms = end.timestamp_millis();
            store.series(h, end_ms - 30 * 60 * 1000, end_ms).ok()
        })
        .map(|rows| rows.iter().map(|r| (r.cpu, r.mem)).unzip())
        .unwrap_or_default();
    HistoryView { start, cpu, mem }
}

/// Runs the TUI until the user quits.
pub async fn run(engine: &Engine, cfg: &Config) -> std::io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut terminal, engine, cfg).await;
    ratatui::restore();
    result
}

async fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    engine: &Engine,
    cfg: &Config,
) -> std::io::Result<()> {
    let mut app = App::default();
    let store = engine.history_path().and_then(|p| Store::open(p).ok());
    let mut events = EventStream::new();
    let mut changes = engine.changes();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    let mut historical: Option<(FleetState, Option<String>)> = None;
    let mut historical_for: Option<(DateTime<Utc>, Option<String>)> = None;
    let mut history = HistoryView::default();
    let mut history_loaded_at: Option<Instant> = None;
    let mut history_key: Option<(Option<String>, TimeCursor)> = None;

    loop {
        let now = Utc::now();
        let live = engine.snapshot();
        let key = (app.selected.clone(), app.time);
        if history_loaded_at.is_none_or(|t| t.elapsed() > Duration::from_secs(5))
            || history_key.as_ref() != Some(&key)
        {
            let end = match app.time {
                TimeCursor::Live => now,
                TimeCursor::At(t) => t,
            };
            let selected = app
                .selected
                .clone()
                .or_else(|| live.hosts.first().map(|h| h.name.clone()));
            history = history_view(store.as_ref(), selected.as_deref(), end);
            app.history_start = history.start;
            history_loaded_at = Some(Instant::now());
            history_key = Some(key);
        }
        let (display, note, at) = match (app.time, &historical) {
            (TimeCursor::At(t), Some((f, n))) => (f.clone(), n.clone(), Some(t)),
            _ => (live.clone(), None, None),
        };
        terminal.draw(|f| {
            let data = ViewData {
                fleet: &display,
                now,
                at,
                thresholds: &cfg.thresholds,
                interval_secs: cfg.interval,
                history: history.clone(),
                note: note.clone(),
            };
            ui::render(f, &mut app, &data);
        })?;

        tokio::select! {
            ev = events.next() => {
                let Some(Ok(ev)) = ev else { break };
                if let Event::Key(k) = ev
                    && k.kind == KeyEventKind::Press
                {
                    match app.handle_key(k, &display.hosts, now) {
                        Action::Quit => break,
                        Action::Snapshot => app.message = Some(take_snapshot(&app, &display, cfg)),
                        Action::TimeChanged | Action::None => {}
                    }
                }
                // Load (or reload) the historical view when the moment or
                // the selected host changed.
                match (app.time, store.as_ref()) {
                    (TimeCursor::At(t), Some(s)) => {
                        let wanted = (t, app.selected.clone());
                        if historical_for.as_ref() != Some(&wanted) {
                            historical = Some(historical_fleet(s, t, &live, app.selected.as_deref()));
                            historical_for = Some(wanted);
                        }
                    }
                    _ => {
                        historical = None;
                        historical_for = None;
                    }
                }
            }
            _ = changes.changed() => {}
            _ = tick.tick() => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Target;

    #[test]
    fn rebuilds_fleet_from_history() {
        let mut store = Store::open_in_memory().unwrap();
        let t0 = Utc::now() - chrono::Duration::minutes(10);
        let ms = t0.timestamp_millis();
        store
            .insert_samples(&[
                SampleRow {
                    host: "a".into(),
                    ts: ms,
                    res: 0,
                    status: 2,
                    cpu: Some(85.0),
                    mem: Some(40.0),
                    disk: Some(10.0),
                    load: Some(0.3),
                    rx: Some(1.0),
                    tx: Some(2.0),
                },
                SampleRow {
                    host: "b".into(),
                    ts: ms,
                    res: 0,
                    status: 4,
                    cpu: None,
                    mem: None,
                    disk: None,
                    load: None,
                    rx: None,
                    tx: None,
                },
            ])
            .unwrap();
        let mut detailed = HostState::new(
            &Target {
                spec: "a".into(),
                groups: vec![],
                allowed_ports: None,
            },
            None,
        );
        detailed.status = HostStatus::Warning;
        store
            .insert_detail("a", ms, &serde_json::to_string(&detailed).unwrap())
            .unwrap();
        let t = |s: &str| Target {
            spec: s.into(),
            groups: vec![],
            allowed_ports: None,
        };
        let live = FleetState {
            hosts: vec![
                HostState::new(&t("a"), None),
                HostState::new(&t("b"), None),
                HostState::new(&t("c"), None),
            ],
            ..Default::default()
        };
        let at = t0 + chrono::Duration::seconds(30);
        let (f, note) = historical_fleet(&store, at, &live, None);
        assert_eq!(f.hosts[0].status, HostStatus::Warning);
        assert_eq!(f.hosts[0].metrics.as_ref().unwrap().cpu_pct(), Some(85.0));
        assert_eq!(f.hosts[1].status, HostStatus::Unreachable);
        assert_eq!(f.hosts[2].status, HostStatus::Pending);
        assert!(note.is_none());
        let (f, note) = historical_fleet(&store, at, &live, Some("a"));
        assert_eq!(f.hosts[0].status, HostStatus::Warning);
        assert!(note.unwrap().starts_with("details recorded"));
    }
}
