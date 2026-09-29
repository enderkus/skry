//! Raw values parsed from the remote output, before any rate computation.

use serde::{Deserialize, Serialize};

/// Result of a probe that may legitimately be unavailable on a host
/// (missing tool, insufficient permission, not collected yet).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum Probe<T> {
    Ok(T),
    Unavailable(String),
    #[default]
    Pending,
}

impl<T> Probe<T> {
    pub fn ok(&self) -> Option<&T> {
        match self {
            Probe::Ok(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Probe::Pending)
    }

    pub fn na(reason: impl Into<String>) -> Self {
        Probe::Unavailable(reason.into())
    }

    /// Short human description for display ("n/a: reason").
    pub fn describe(&self) -> Option<String> {
        match self {
            Probe::Ok(_) => None,
            Probe::Unavailable(r) => Some(format!("n/a ({r})")),
            Probe::Pending => Some("…".into()),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Probe<U> {
        match self {
            Probe::Ok(v) => Probe::Ok(f(v)),
            Probe::Unavailable(r) => Probe::Unavailable(r),
            Probe::Pending => Probe::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Identity {
    pub hostname: String,
    pub kernel: String,
    pub arch: String,
    /// Remote wall clock, seconds since the epoch.
    pub remote_time: Option<i64>,
    /// Remote local time truncated to the hour, `YYYY-MM-DDTHH`.
    pub local_hour: Option<String>,
    pub page_size: u64,
    pub clk_tck: u64,
    pub uid: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OsRelease {
    pub id: String,
    pub id_like: String,
    pub name: String,
    pub pretty_name: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuCounters {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuCounters {
    /// Total jiffies. Guest time is already included in user/nice.
    pub fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    pub fn idle_all(&self) -> u64 {
        self.idle + self.iowait
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuStat {
    pub total: CpuCounters,
    pub cores: Vec<CpuCounters>,
    pub btime: Option<u64>,
    pub procs_running: Option<u64>,
    pub procs_blocked: Option<u64>,
}

/// Memory figures in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemInfo {
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub buffers: u64,
    pub cached: u64,
    pub sreclaimable: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
    pub running: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetDevCounters {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errs: u64,
    pub rx_drop: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errs: u64,
    pub tx_drop: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskStatCounters {
    pub major: u32,
    pub minor: u32,
    pub name: String,
    pub reads: u64,
    pub sectors_read: u64,
    pub writes: u64,
    pub sectors_written: u64,
    /// Milliseconds spent doing I/O.
    pub io_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DfEntry {
    pub filesystem: String,
    pub total_kb: u64,
    pub used_kb: u64,
    pub avail_kb: u64,
    pub mount: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpAddrEntry {
    pub iface: String,
    pub family: String,
    pub cidr: String,
    pub scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcStat {
    pub pid: u32,
    pub state: String,
    pub utime: u64,
    pub stime: u64,
    pub rss_pages: u64,
    pub comm: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcArgs {
    pub pid: u32,
    pub user: String,
    pub args: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ListenPort {
    pub port: u16,
    pub addr: String,
}

impl std::fmt::Display for ListenPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.addr.contains(':') {
            write!(f, "[{}]:{}", self.addr, self.port)
        } else {
            write!(f, "{}:{}", self.addr, self.port)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Container {
    pub runtime: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub cpu_pct: Option<f64>,
    pub mem_usage: Option<String>,
    pub mem_pct: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedUnit {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitStatus {
    pub id: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedLogins {
    /// `journal` or the log file path that was read.
    pub source: String,
    pub total: u64,
    /// Source address and event count, most active first.
    pub top_sources: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Updates {
    pub manager: String,
    pub total: u64,
    /// `None` when the package manager exposes no security metadata.
    pub security: Option<u64>,
    pub security_packages: Vec<String>,
}
