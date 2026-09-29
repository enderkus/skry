//! Local history in SQLite, with automatic downsampling.
//!
//! Two tables hold history:
//!
//! * `samples` — one small summary row per host per tick (status, CPU,
//!   memory, disk, load, network), used for fleet time travel and timelines.
//!   Rows older than an hour are folded into one-minute buckets, and rows
//!   older than six hours into five-minute buckets.
//! * `details` — the full host state as JSON every `detail_interval`
//!   seconds, used for the host view when scrubbing back. Older than an hour,
//!   one record per five minutes is kept.
//!
//! Everything older than the retention period is deleted.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("history database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("history database: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

const SCHEMA_VERSION: i64 = 1;
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;

/// Severity stored with each sample; higher is worse. Matches
/// [`crate::engine::HostStatus::rank`].
pub type StatusRank = i64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleRow {
    pub host: String,
    /// Unix milliseconds.
    pub ts: i64,
    /// Bucket width in seconds: 0 for raw samples, 60 or 300 when downsampled.
    pub res: i64,
    pub status: StatusRank,
    pub cpu: Option<f64>,
    pub mem: Option<f64>,
    pub disk: Option<f64>,
    pub load: Option<f64>,
    pub rx: Option<f64>,
    pub tx: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineRow {
    pub host: String,
    pub metric: String,
    /// 0–23 for hour-of-day buckets, 24 for the global baseline.
    pub bucket: i64,
    pub mean: f64,
    pub var: f64,
    pub n: i64,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS samples (
                 host TEXT NOT NULL, ts INTEGER NOT NULL, res INTEGER NOT NULL DEFAULT 0,
                 status INTEGER NOT NULL, cpu REAL, mem REAL, disk REAL, load REAL, rx REAL, tx REAL);
             CREATE INDEX IF NOT EXISTS samples_host_ts ON samples(host, ts);
             CREATE INDEX IF NOT EXISTS samples_ts ON samples(ts);
             CREATE TABLE IF NOT EXISTS details (host TEXT NOT NULL, ts INTEGER NOT NULL, json TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS details_host_ts ON details(host, ts);
             CREATE TABLE IF NOT EXISTS baselines (
                 host TEXT NOT NULL, metric TEXT NOT NULL, bucket INTEGER NOT NULL,
                 mean REAL NOT NULL, var REAL NOT NULL, n INTEGER NOT NULL,
                 PRIMARY KEY (host, metric, bucket));",
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;
        Ok(Store { conn })
    }

    pub fn insert_samples(&mut self, rows: &[SampleRow]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO samples (host, ts, res, status, cpu, mem, disk, load, rx, tx)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for r in rows {
                stmt.execute(params![
                    r.host, r.ts, r.res, r.status, r.cpu, r.mem, r.disk, r.load, r.rx, r.tx
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_detail(&mut self, host: &str, ts: i64, json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO details (host, ts, json) VALUES (?1, ?2, ?3)",
            params![host, ts, json],
        )?;
        Ok(())
    }

    /// Downsamples old data and enforces retention. `now` is Unix ms.
    pub fn maintain(&mut self, now: i64, retention_hours: u64) -> Result<()> {
        let tx = self.conn.transaction()?;
        let fold = |tx: &rusqlite::Transaction, from_res: i64, to_res: i64, older_than: i64| {
            let width = to_res * 1000;
            // Only fold buckets that are complete.
            let cutoff = (now - older_than) / width * width;
            tx.execute(
                "INSERT INTO samples (host, ts, res, status, cpu, mem, disk, load, rx, tx)
                 SELECT host, (ts / ?1) * ?1, ?2, MAX(status), AVG(cpu), AVG(mem), AVG(disk),
                        AVG(load), AVG(rx), AVG(tx)
                 FROM samples WHERE res = ?3 AND ts < ?4
                 GROUP BY host, ts / ?1",
                params![width, to_res, from_res, cutoff],
            )?;
            tx.execute(
                "DELETE FROM samples WHERE res = ?1 AND ts < ?2",
                params![from_res, cutoff],
            )
        };
        fold(&tx, 0, 60, HOUR_MS)?;
        fold(&tx, 60, 300, 6 * HOUR_MS)?;
        let cutoff = now - HOUR_MS;
        tx.execute(
            "DELETE FROM details WHERE ts < ?1 AND rowid NOT IN (
                 SELECT MIN(rowid) FROM details WHERE ts < ?1 GROUP BY host, ts / 300000)",
            params![cutoff],
        )?;
        let keep_from = now - retention_hours as i64 * HOUR_MS;
        tx.execute("DELETE FROM samples WHERE ts < ?1", params![keep_from])?;
        tx.execute("DELETE FROM details WHERE ts < ?1", params![keep_from])?;
        tx.commit()?;
        Ok(())
    }

    fn row(r: &rusqlite::Row) -> rusqlite::Result<SampleRow> {
        Ok(SampleRow {
            host: r.get(0)?,
            ts: r.get(1)?,
            res: r.get(2)?,
            status: r.get(3)?,
            cpu: r.get(4)?,
            mem: r.get(5)?,
            disk: r.get(6)?,
            load: r.get(7)?,
            rx: r.get(8)?,
            tx: r.get(9)?,
        })
    }

    /// The latest sample of every host at or before `ts`, looking back at
    /// most `window_ms`.
    pub fn fleet_at(&self, ts: i64, window_ms: i64) -> Result<Vec<SampleRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT s.host, s.ts, s.res, s.status, s.cpu, s.mem, s.disk, s.load, s.rx, s.tx
             FROM samples s JOIN (
                 SELECT host, MAX(ts) AS mts FROM samples WHERE ts <= ?1 AND ts >= ?2 GROUP BY host
             ) m ON s.host = m.host AND s.ts = m.mts
             GROUP BY s.host ORDER BY s.host",
        )?;
        let rows = stmt.query_map(params![ts, ts - window_ms], Self::row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Samples for one host in `[from, to]`, oldest first.
    pub fn series(&self, host: &str, from: i64, to: i64) -> Result<Vec<SampleRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT host, ts, res, status, cpu, mem, disk, load, rx, tx FROM samples
             WHERE host = ?1 AND ts >= ?2 AND ts <= ?3 ORDER BY ts",
        )?;
        let rows = stmt.query_map(params![host, from, to], Self::row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The latest full host record at or before `ts`, with its timestamp.
    pub fn detail_at(&self, host: &str, ts: i64, window_ms: i64) -> Result<Option<(i64, String)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT ts, json FROM details WHERE host = ?1 AND ts <= ?2 AND ts >= ?3
                 ORDER BY ts DESC LIMIT 1",
                params![host, ts, ts - window_ms],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// Oldest and newest sample timestamps.
    pub fn time_range(&self) -> Result<Option<(i64, i64)>> {
        Ok(self
            .conn
            .query_row("SELECT MIN(ts), MAX(ts) FROM samples", [], |r| {
                Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<i64>>(1)?))
            })
            .map(|(a, b)| a.zip(b))?)
    }

    pub fn save_baselines(&mut self, rows: &[BaselineRow]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO baselines (host, metric, bucket, mean, var, n)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for r in rows {
                stmt.execute(params![r.host, r.metric, r.bucket, r.mean, r.var, r.n])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_baselines(&self) -> Result<Vec<BaselineRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT host, metric, bucket, mean, var, n FROM baselines")?;
        let rows = stmt.query_map([], |r| {
            Ok(BaselineRow {
                host: r.get(0)?,
                metric: r.get(1)?,
                bucket: r.get(2)?,
                mean: r.get(3)?,
                var: r.get(4)?,
                n: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn count_samples(&self, res: i64) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM samples WHERE res = ?1",
            params![res],
            |r| r.get(0),
        )?)
    }

    pub fn count_details(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM details", [], |r| r.get(0))?)
    }
}

/// Messages for the writer thread.
pub enum StoreMsg {
    Samples(Vec<SampleRow>),
    Detail { host: String, ts: i64, json: String },
    Baselines(Vec<BaselineRow>),
    Flush(std::sync::mpsc::Sender<()>),
}

/// Runs a dedicated writer thread so that SQLite never blocks the async
/// runtime. Maintenance runs every five minutes.
pub fn spawn_writer(
    mut store: Store,
    retention_hours: u64,
) -> (
    std::sync::mpsc::Sender<StoreMsg>,
    std::thread::JoinHandle<()>,
) {
    let (tx, rx) = std::sync::mpsc::channel::<StoreMsg>();
    let handle = std::thread::Builder::new()
        .name("skry-store".into())
        .spawn(move || {
            let now_ms = || chrono::Utc::now().timestamp_millis();
            let mut last_maintain = 0i64;
            loop {
                let msg = match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(m) => Some(m),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let res = match msg {
                    Some(StoreMsg::Samples(rows)) => store.insert_samples(&rows),
                    Some(StoreMsg::Detail { host, ts, json }) => {
                        store.insert_detail(&host, ts, &json)
                    }
                    Some(StoreMsg::Baselines(rows)) => store.save_baselines(&rows),
                    Some(StoreMsg::Flush(done)) => {
                        let _ = done.send(());
                        Ok(())
                    }
                    None => Ok(()),
                };
                if let Err(e) = res {
                    tracing::warn!(error = %e, "history write failed");
                }
                let now = now_ms();
                if now - last_maintain > 5 * MINUTE_MS {
                    last_maintain = now;
                    if let Err(e) = store.maintain(now, retention_hours) {
                        tracing::warn!(error = %e, "history maintenance failed");
                    }
                }
            }
        })
        .expect("spawn store thread");
    (tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(host: &str, ts: i64, cpu: f64, status: i64) -> SampleRow {
        SampleRow {
            host: host.into(),
            ts,
            res: 0,
            status,
            cpu: Some(cpu),
            mem: Some(50.0),
            disk: None,
            load: Some(0.5),
            rx: Some(100.0),
            tx: Some(10.0),
        }
    }

    const T0: i64 = 1_790_000_000_000 / HOUR_MS * HOUR_MS;

    #[test]
    fn fleet_at_picks_latest_before() {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_samples(&[
            sample("a", T0, 10.0, 0),
            sample("a", T0 + 2000, 20.0, 0),
            sample("b", T0 + 1000, 30.0, 4),
            sample("a", T0 + 4000, 40.0, 0),
        ])
        .unwrap();
        let rows = s.fleet_at(T0 + 3000, 60_000).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].cpu, Some(20.0));
        assert_eq!(rows[1].status, 4);
        // Outside the window nothing is returned.
        assert!(s.fleet_at(T0 + 3_000_000, 60_000).unwrap().is_empty());
    }

    #[test]
    fn downsampling_folds_old_samples() {
        let mut s = Store::open_in_memory().unwrap();
        // Two hours of 2-second samples for one host.
        let rows: Vec<SampleRow> = (0..3600)
            .map(|i| {
                sample(
                    "a",
                    T0 + i * 2000,
                    (i % 10) as f64,
                    if i == 5 { 3 } else { 0 },
                )
            })
            .collect();
        s.insert_samples(&rows).unwrap();
        let now = T0 + 2 * HOUR_MS;
        s.maintain(now, 24).unwrap();
        // The first hour became 60 one-minute buckets; the last hour is raw.
        assert_eq!(s.count_samples(60).unwrap(), 60);
        assert_eq!(s.count_samples(0).unwrap(), 1800);
        let first = s.series("a", T0, T0).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].res, 60);
        assert_eq!(first[0].cpu, Some(4.5));
        assert_eq!(first[0].status, 3, "bucket keeps the worst status");
        // Running again is idempotent.
        s.maintain(now, 24).unwrap();
        assert_eq!(s.count_samples(60).unwrap(), 60);

        // Eight hours later: minute buckets older than 6 h become 5-minute buckets.
        s.maintain(T0 + 8 * HOUR_MS, 24).unwrap();
        assert_eq!(s.count_samples(0).unwrap(), 0);
        assert_eq!(s.count_samples(300).unwrap(), 12 + 12);
    }

    #[test]
    fn retention_deletes_everything_old() {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_samples(&[sample("a", T0, 1.0, 0)]).unwrap();
        s.insert_detail("a", T0, "{}").unwrap();
        s.maintain(T0 + 25 * HOUR_MS, 24).unwrap();
        assert_eq!(s.time_range().unwrap(), None);
        assert_eq!(s.count_details().unwrap(), 0);
    }

    #[test]
    fn details_thinned_after_an_hour() {
        let mut s = Store::open_in_memory().unwrap();
        for i in 0..240 {
            s.insert_detail("a", T0 + i * 30_000, &format!("{{\"i\":{i}}}"))
                .unwrap();
        }
        s.maintain(T0 + 2 * HOUR_MS, 24).unwrap();
        // 120 records in the first hour thin to 12 (one per 5 min); the rest stay.
        assert_eq!(s.count_details().unwrap(), 12 + 120);
        let (ts, json) = s.detail_at("a", T0 + 7 * 60_000, HOUR_MS).unwrap().unwrap();
        assert_eq!(ts, T0 + 5 * 60_000);
        assert_eq!(json, "{\"i\":10}");
    }

    #[test]
    fn baselines_roundtrip() {
        let mut s = Store::open_in_memory().unwrap();
        let row = BaselineRow {
            host: "a".into(),
            metric: "cpu".into(),
            bucket: 24,
            mean: 12.5,
            var: 4.0,
            n: 300,
        };
        s.save_baselines(std::slice::from_ref(&row)).unwrap();
        s.save_baselines(&[BaselineRow {
            n: 301,
            ..row.clone()
        }])
        .unwrap();
        let rows = s.load_baselines().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].n, 301);
    }

    #[test]
    fn open_file_and_writer_thread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/history.db");
        let store = Store::open(&path).unwrap();
        let (tx, handle) = spawn_writer(store, 24);
        let now = chrono::Utc::now().timestamp_millis();
        tx.send(StoreMsg::Samples(vec![sample("a", now, 1.0, 0)]))
            .unwrap();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        tx.send(StoreMsg::Flush(done_tx)).unwrap();
        done_rx.recv().unwrap();
        drop(tx);
        handle.join().unwrap();
        let reader = Store::open(&path).unwrap();
        assert_eq!(reader.fleet_at(now, 1000).unwrap().len(), 1);
    }
}
