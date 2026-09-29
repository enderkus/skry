//! Statistical baselines and deviation detection.
//!
//! For every host and metric an exponentially weighted moving average of
//! the mean and variance is maintained, globally and (optionally) per hour
//! of the day. Once a baseline has seen `warmup` samples, a value whose
//! z-score exceeds the threshold is reported as a deviation. Only upward
//! deviations are reported: a server that is suddenly idle is rarely an
//! incident.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::BaselineConfig;
use crate::model::HostMetrics;
use crate::store::BaselineRow;

/// Incremental EWMA estimate of mean and variance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Ewma {
    pub mean: f64,
    pub var: f64,
    pub n: u64,
}

impl Ewma {
    pub fn update(&mut self, x: f64, alpha: f64) {
        if self.n == 0 {
            self.mean = x;
            self.var = 0.0;
        } else {
            let diff = x - self.mean;
            let incr = alpha * diff;
            self.mean += incr;
            self.var = (1.0 - alpha) * (self.var + diff * incr);
        }
        self.n = self.n.saturating_add(1);
    }

    pub fn std(&self) -> f64 {
        self.var.max(0.0).sqrt()
    }

    /// z-score of `x`, with the standard deviation floored at `min_std` so
    /// that perfectly flat metrics do not produce infinite scores.
    pub fn z(&self, x: f64, min_std: f64) -> f64 {
        (x - self.mean) / self.std().max(min_std)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Cpu,
    Memory,
    Load,
    Disk,
    NetRx,
    NetTx,
}

impl Metric {
    pub const ALL: [Metric; 6] = [
        Metric::Cpu,
        Metric::Memory,
        Metric::Load,
        Metric::Disk,
        Metric::NetRx,
        Metric::NetTx,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Metric::Cpu => "cpu",
            Metric::Memory => "memory",
            Metric::Load => "load",
            Metric::Disk => "disk",
            Metric::NetRx => "net_rx",
            Metric::NetTx => "net_tx",
        }
    }

    pub fn from_name(s: &str) -> Option<Metric> {
        Metric::ALL.into_iter().find(|m| m.name() == s)
    }

    pub fn value(self, m: &HostMetrics) -> Option<f64> {
        match self {
            Metric::Cpu => m.cpu_pct(),
            Metric::Memory => m.mem_pct(),
            Metric::Load => m.load_per_core,
            Metric::Disk => m.disk_max_pct(),
            Metric::NetRx => m.net_rx_bps,
            Metric::NetTx => m.net_tx_bps,
        }
    }

    /// Minimum standard deviation, in the metric's unit, so that tiny
    /// wobbles on a quiet metric are not flagged.
    fn min_std(self, mean: f64) -> f64 {
        match self {
            Metric::Cpu => 5.0,
            Metric::Memory => 2.0,
            Metric::Load => 0.25,
            Metric::Disk => 1.0,
            Metric::NetRx | Metric::NetTx => (mean.abs() * 0.25).max(64.0 * 1024.0),
        }
    }

    pub fn format(self, v: f64) -> String {
        match self {
            Metric::Cpu | Metric::Memory | Metric::Disk => format!("{v:.1}%"),
            Metric::Load => format!("{v:.2}/core"),
            Metric::NetRx | Metric::NetTx => format!("{}/s", crate::util::human_bytes(v)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deviation {
    pub metric: Metric,
    pub value: f64,
    pub mean: f64,
    pub std: f64,
    pub z: f64,
}

impl Deviation {
    pub fn describe(&self) -> String {
        format!(
            "{} {} vs usual {} (z={:.1})",
            self.metric.name(),
            self.metric.format(self.value),
            self.metric.format(self.mean),
            self.z
        )
    }
}

#[derive(Debug, Clone, Default)]
struct MetricBaseline {
    global: Ewma,
    hourly: [Ewma; 24],
}

/// Baselines for all hosts.
#[derive(Debug, Clone)]
pub struct Baselines {
    cfg: BaselineConfig,
    map: HashMap<(String, Metric), MetricBaseline>,
}

impl Baselines {
    pub fn new(cfg: BaselineConfig) -> Self {
        Self {
            cfg,
            map: HashMap::new(),
        }
    }

    /// Folds one sample in and returns the deviations it shows. `hour` is
    /// the local hour of day (0–23).
    pub fn observe(&mut self, host: &str, m: &HostMetrics, hour: u32) -> Vec<Deviation> {
        let mut out = Vec::new();
        if !self.cfg.enabled {
            return out;
        }
        let hour = (hour % 24) as usize;
        for metric in Metric::ALL {
            let Some(x) = metric.value(m) else { continue };
            if !x.is_finite() {
                continue;
            }
            let b = self.map.entry((host.to_string(), metric)).or_default();
            let hourly = &b.hourly[hour];
            let reference = if self.cfg.hourly && hourly.n >= self.cfg.warmup {
                *hourly
            } else {
                b.global
            };
            if reference.n >= self.cfg.warmup {
                let z = reference.z(x, metric.min_std(reference.mean));
                if z > self.cfg.z_threshold {
                    out.push(Deviation {
                        metric,
                        value: x,
                        mean: reference.mean,
                        std: reference.std(),
                        z,
                    });
                }
            }
            b.global.update(x, self.cfg.alpha);
            if self.cfg.hourly {
                b.hourly[hour].update(x, self.cfg.alpha);
            }
        }
        out
    }

    /// Whether the global baseline for this host has finished warming up.
    pub fn warmed_up(&self, host: &str) -> bool {
        self.map
            .get(&(host.to_string(), Metric::Cpu))
            .is_some_and(|b| b.global.n >= self.cfg.warmup)
    }

    pub fn export(&self) -> Vec<BaselineRow> {
        let mut rows = Vec::new();
        for ((host, metric), b) in &self.map {
            let mut push = |bucket: i64, e: &Ewma| {
                if e.n > 0 {
                    rows.push(BaselineRow {
                        host: host.clone(),
                        metric: metric.name().to_string(),
                        bucket,
                        mean: e.mean,
                        var: e.var,
                        n: e.n as i64,
                    });
                }
            };
            push(24, &b.global);
            for (h, e) in b.hourly.iter().enumerate() {
                push(h as i64, e);
            }
        }
        rows
    }

    pub fn import(&mut self, rows: &[BaselineRow]) {
        for r in rows {
            let Some(metric) = Metric::from_name(&r.metric) else {
                continue;
            };
            let e = Ewma {
                mean: r.mean,
                var: r.var,
                n: r.n.max(0) as u64,
            };
            let b = self.map.entry((r.host.clone(), metric)).or_default();
            match r.bucket {
                24 => b.global = e,
                h @ 0..=23 => b.hourly[h as usize] = e,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CpuUsage;

    fn cpu(v: f64) -> HostMetrics {
        HostMetrics {
            cpu: Some(CpuUsage {
                total_pct: v,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn cfg() -> BaselineConfig {
        BaselineConfig {
            enabled: true,
            alpha: 0.05,
            z_threshold: 3.0,
            warmup: 50,
            hourly: false,
        }
    }

    #[test]
    fn ewma_converges_to_mean_and_variance() {
        let mut e = Ewma::default();
        // Alternating 10/20: mean 15, variance 25.
        for i in 0..5000 {
            e.update(if i % 2 == 0 { 10.0 } else { 20.0 }, 0.01);
        }
        assert!((e.mean - 15.0).abs() < 0.5, "mean {}", e.mean);
        assert!((e.var - 25.0).abs() < 2.0, "var {}", e.var);
        assert!((e.z(30.0, 0.0) - 3.0).abs() < 0.2);
    }

    #[test]
    fn first_value_initialises() {
        let mut e = Ewma::default();
        e.update(42.0, 0.1);
        assert_eq!((e.mean, e.var, e.n), (42.0, 0.0, 1));
    }

    #[test]
    fn min_std_prevents_division_by_zero() {
        let e = Ewma {
            mean: 10.0,
            var: 0.0,
            n: 100,
        };
        assert_eq!(e.z(20.0, 5.0), 2.0);
    }

    #[test]
    fn no_deviation_during_warmup() {
        let mut b = Baselines::new(cfg());
        for _ in 0..49 {
            assert!(b.observe("h", &cpu(5.0), 0).is_empty());
        }
        assert!(!b.warmed_up("h"));
        // Sample 50 is still judged against a 49-sample baseline.
        assert!(b.observe("h", &cpu(99.0), 0).is_empty());
        assert!(b.warmed_up("h"));
    }

    #[test]
    fn spike_after_warmup_is_flagged_once_baseline_adapts() {
        let mut b = Baselines::new(cfg());
        for i in 0..200 {
            b.observe("h", &cpu(10.0 + (i % 5) as f64), 0);
        }
        let d = b.observe("h", &cpu(60.0), 0);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].metric, Metric::Cpu);
        assert!(d[0].z > 3.0);
        assert!(d[0].describe().starts_with("cpu 60.0% vs usual"));
        // A value within normal variation is not flagged.
        assert!(b.observe("h", &cpu(13.0), 0).is_empty());
        // Downward moves are never flagged.
        assert!(b.observe("h", &cpu(0.0), 0).is_empty());
        // Other hosts are independent.
        assert!(b.observe("other", &cpu(60.0), 0).is_empty());
    }

    #[test]
    fn hourly_buckets_learn_daily_patterns() {
        let mut c = cfg();
        c.hourly = true;
        let mut b = Baselines::new(c);
        // Busy at 03:00 (backups), quiet otherwise.
        for _ in 0..100 {
            b.observe("h", &cpu(80.0), 3);
            b.observe("h", &cpu(5.0), 12);
        }
        assert!(b.observe("h", &cpu(80.0), 3).is_empty(), "normal for 03:00");
        assert_eq!(b.observe("h", &cpu(80.0), 12).len(), 1, "unusual for noon");
    }

    #[test]
    fn disabled_reports_nothing() {
        let mut c = cfg();
        c.enabled = false;
        let mut b = Baselines::new(c);
        for _ in 0..100 {
            b.observe("h", &cpu(1.0), 0);
        }
        assert!(b.observe("h", &cpu(100.0), 0).is_empty());
    }

    #[test]
    fn export_import_roundtrip() {
        let mut c = cfg();
        c.hourly = true;
        let mut b = Baselines::new(c.clone());
        for _ in 0..60 {
            b.observe("h", &cpu(10.0), 7);
        }
        let rows = b.export();
        assert_eq!(rows.len(), 2);
        let mut b2 = Baselines::new(c);
        b2.import(&rows);
        assert!(b2.warmed_up("h"));
        assert_eq!(b2.observe("h", &cpu(90.0), 7).len(), 1);
    }
}
