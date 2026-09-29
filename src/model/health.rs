//! Static health thresholds.

use serde::{Deserialize, Serialize};

use super::HostMetrics;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    #[default]
    Ok,
    Warning,
    Critical,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Ok => "ok",
            Level::Warning => "warning",
            Level::Critical => "critical",
        }
    }
}

/// Warning and critical limits for one metric.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub warning: f64,
    pub critical: f64,
}

impl Band {
    pub const fn new(warning: f64, critical: f64) -> Self {
        Self { warning, critical }
    }

    pub fn level(&self, value: f64) -> Level {
        if value >= self.critical {
            Level::Critical
        } else if value >= self.warning {
            Level::Warning
        } else {
            Level::Ok
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Thresholds {
    /// CPU busy percentage.
    pub cpu: Band,
    /// Memory used percentage (excluding reclaimable cache).
    pub memory: Band,
    /// Highest filesystem usage percentage.
    pub disk: Band,
    /// One-minute load average divided by the number of cores.
    pub load_per_core: Band,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            cpu: Band::new(80.0, 95.0),
            memory: Band::new(85.0, 95.0),
            disk: Band::new(80.0, 90.0),
            load_per_core: Band::new(1.5, 3.0),
        }
    }
}

/// A metric that crossed a threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Breach {
    pub metric: String,
    pub level: Level,
    pub value: f64,
    pub limit: f64,
}

impl Breach {
    pub fn describe(&self) -> String {
        let unit = if self.metric == "load_per_core" {
            ""
        } else {
            "%"
        };
        format!(
            "{} {:.1}{unit} ≥ {:.1}{unit} ({})",
            self.metric,
            self.value,
            self.limit,
            self.level.as_str()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub level: Level,
    pub breaches: Vec<Breach>,
}

impl Health {
    pub fn evaluate(m: &HostMetrics, t: &Thresholds) -> Health {
        let checks = [
            ("cpu", m.cpu_pct(), t.cpu),
            ("memory", m.mem_pct(), t.memory),
            ("disk", m.disk_max_pct(), t.disk),
            ("load_per_core", m.load_per_core, t.load_per_core),
        ];
        let mut breaches = Vec::new();
        for (metric, value, band) in checks {
            let Some(value) = value else { continue };
            let level = band.level(value);
            if level != Level::Ok {
                let limit = if level == Level::Critical {
                    band.critical
                } else {
                    band.warning
                };
                breaches.push(Breach {
                    metric: metric.to_string(),
                    level,
                    value,
                    limit,
                });
            }
        }
        let level = breaches.iter().map(|b| b.level).max().unwrap_or_default();
        Health { level, breaches }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CpuUsage, DiskUsage, MemUsage};

    #[test]
    fn levels() {
        let b = Band::new(80.0, 90.0);
        assert_eq!(b.level(79.9), Level::Ok);
        assert_eq!(b.level(80.0), Level::Warning);
        assert_eq!(b.level(95.0), Level::Critical);
    }

    #[test]
    fn evaluate_picks_worst() {
        let m = HostMetrics {
            cpu: Some(CpuUsage {
                total_pct: 85.0,
                ..Default::default()
            }),
            mem: Some(MemUsage {
                used_pct: 10.0,
                ..Default::default()
            }),
            disks: vec![
                DiskUsage {
                    used_pct: 50.0,
                    ..Default::default()
                },
                DiskUsage {
                    used_pct: 93.0,
                    ..Default::default()
                },
            ],
            load_per_core: Some(0.2),
            ..Default::default()
        };
        let h = Health::evaluate(&m, &Thresholds::default());
        assert_eq!(h.level, Level::Critical);
        assert_eq!(h.breaches.len(), 2);
        assert_eq!(h.breaches[0].metric, "cpu");
        assert_eq!(h.breaches[1].describe(), "disk 93.0% ≥ 90.0% (critical)");
    }

    #[test]
    fn missing_metrics_are_ok() {
        let h = Health::evaluate(&HostMetrics::default(), &Thresholds::default());
        assert_eq!(h.level, Level::Ok);
    }
}
