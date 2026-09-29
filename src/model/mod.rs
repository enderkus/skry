//! Metric types and rate computation.
//!
//! Remote counters (`/proc/stat`, `/proc/net/dev`, `/proc/diskstats`,
//! per-process CPU time) are monotonically increasing; rates and
//! percentages are derived from the delta between two consecutive samples.

pub mod health;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::collect::{
    Container, CpuCounters, DfEntry, DiskStatCounters, FailedLogins, FailedUnit, IpAddrEntry,
    ListenPort, LoadAvg, NetDevCounters, Probe, RawSample, Updates,
};

pub use health::{Health, Level, Thresholds};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CpuUsage {
    pub total_pct: f64,
    pub user_pct: f64,
    pub system_pct: f64,
    pub iowait_pct: f64,
    pub steal_pct: f64,
    pub cores: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub cache: u64,
    pub used_pct: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_pct: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetRate {
    pub iface: String,
    /// Bytes per second.
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_total: u64,
    pub tx_total: u64,
    pub errors: u64,
    pub drops: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskUsage {
    pub mount: String,
    pub filesystem: String,
    pub total: u64,
    pub used: u64,
    pub avail: u64,
    pub used_pct: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskIoRate {
    pub device: String,
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub util_pct: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcInfo {
    pub pid: u32,
    pub user: String,
    pub name: String,
    pub state: String,
    /// Percent of one core; `None` until two samples are available.
    pub cpu_pct: Option<f64>,
    pub mem_pct: f64,
    pub rss: u64,
}

/// Everything skry knows about one host at one moment.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HostMetrics {
    pub ts: DateTime<Utc>,
    pub hostname: String,
    pub os: Option<String>,
    pub os_id: Option<String>,
    pub kernel: String,
    pub arch: String,
    pub cpu_model: Option<String>,
    pub cores: usize,
    pub uptime_secs: Option<f64>,
    pub cpu: Option<CpuUsage>,
    pub mem: Option<MemUsage>,
    pub load: Option<LoadAvg>,
    pub load_per_core: Option<f64>,
    pub net: Vec<NetRate>,
    pub net_rx_bps: Option<f64>,
    pub net_tx_bps: Option<f64>,
    pub disks: Vec<DiskUsage>,
    pub disk_io: Vec<DiskIoRate>,
    pub ips: Vec<IpAddrEntry>,
    pub procs: Vec<ProcInfo>,
    pub proc_count: usize,
    #[serde(default)]
    pub ports: Probe<Vec<ListenPort>>,
    #[serde(default)]
    pub containers: Probe<Vec<Container>>,
    #[serde(default)]
    pub failed_units: Probe<Vec<FailedUnit>>,
    #[serde(default)]
    pub failed_logins: Probe<FailedLogins>,
    #[serde(default)]
    pub updates: Probe<Updates>,
}

impl HostMetrics {
    /// Highest usage percentage over all real filesystems.
    pub fn disk_max_pct(&self) -> Option<f64> {
        self.disks.iter().map(|d| d.used_pct).reduce(f64::max)
    }

    pub fn cpu_pct(&self) -> Option<f64> {
        self.cpu.as_ref().map(|c| c.total_pct)
    }

    pub fn mem_pct(&self) -> Option<f64> {
        self.mem.as_ref().map(|m| m.used_pct)
    }
}

fn pct(part: f64, whole: f64) -> f64 {
    if whole <= 0.0 {
        0.0
    } else {
        (part / whole * 100.0).clamp(0.0, 100.0)
    }
}

fn cpu_delta(prev: &CpuCounters, cur: &CpuCounters) -> Option<(f64, f64, f64, f64, f64)> {
    let total = cur.total().checked_sub(prev.total())? as f64;
    if total <= 0.0 {
        return None;
    }
    let d = |a: u64, b: u64| b.saturating_sub(a) as f64;
    let idle = d(prev.idle_all(), cur.idle_all());
    let user = d(prev.user + prev.nice, cur.user + cur.nice);
    let system = d(
        prev.system + prev.irq + prev.softirq,
        cur.system + cur.irq + cur.softirq,
    );
    Some((
        pct(total - idle, total),
        pct(user, total),
        pct(system, total),
        pct(d(prev.iowait, cur.iowait), total),
        pct(d(prev.steal, cur.steal), total),
    ))
}

/// Pseudo and container-internal filesystems that are noise in a disk view.
fn interesting_fs(e: &DfEntry) -> bool {
    const PSEUDO_FS: &[&str] = &[
        "tmpfs",
        "devtmpfs",
        "udev",
        "shm",
        "none",
        "efivarfs",
        "proc",
        "sysfs",
        "cgroup",
        "cgroup2",
        "devpts",
        "mqueue",
        "overlayfs-meta",
    ];
    const SKIP_MOUNTS: &[&str] = &[
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/snap/",
        "/var/lib/docker/",
        "/var/lib/containers/",
        "/var/lib/kubelet/",
    ];
    if e.total_kb == 0 || PSEUDO_FS.contains(&e.filesystem.as_str()) {
        return false;
    }
    if e.filesystem.starts_with("/dev/loop") {
        return false;
    }
    !SKIP_MOUNTS.iter().any(|base| {
        let base = base.trim_end_matches('/');
        e.mount == base
            || e.mount
                .strip_prefix(base)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

pub fn disk_usage(df: &[DfEntry]) -> Vec<DiskUsage> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<DiskUsage> = Vec::new();
    for e in df.iter().filter(|e| interesting_fs(e)) {
        let usage = DiskUsage {
            mount: e.mount.clone(),
            filesystem: e.filesystem.clone(),
            total: e.total_kb * 1024,
            used: e.used_kb * 1024,
            avail: e.avail_kb * 1024,
            // Same formula as df: reserved blocks do not count as available.
            used_pct: pct(e.used_kb as f64, (e.used_kb + e.avail_kb) as f64),
        };
        // Bind mounts show the same device several times; keep the shortest path.
        match seen.get(e.filesystem.as_str()) {
            Some(&i) if e.filesystem.starts_with('/') => {
                if usage.mount.len() < out[i].mount.len() {
                    out[i] = usage;
                }
            }
            _ => {
                seen.insert(&e.filesystem, out.len());
                out.push(usage);
            }
        }
    }
    out.sort_by(|a, b| a.mount.cmp(&b.mount));
    out
}

fn is_virtual_disk(name: &str) -> bool {
    ["loop", "ram", "zram", "nbd", "sr", "fd"]
        .iter()
        .any(|p| name.starts_with(p))
}

/// True for partitions such as `sda1`, `nvme0n1p2`, `mmcblk0p1`.
fn is_partition(name: &str, all: &[DiskStatCounters]) -> bool {
    if !name.ends_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    all.iter().any(|d| {
        d.name != name
            && name.starts_with(&d.name)
            && name[d.name.len()..]
                .trim_start_matches('p')
                .chars()
                .all(|c| c.is_ascii_digit())
            && !name[d.name.len()..].trim_start_matches('p').is_empty()
    })
}

pub fn physical_disks(all: &[DiskStatCounters]) -> Vec<&DiskStatCounters> {
    all.iter()
        .filter(|d| !is_virtual_disk(&d.name) && !is_partition(&d.name, all))
        .filter(|d| d.reads + d.writes > 0)
        .collect()
}

/// Interfaces that carry host traffic, as opposed to container plumbing.
pub fn is_primary_iface(name: &str) -> bool {
    const SKIP: &[&str] = &[
        "lo", "veth", "docker", "br-", "virbr", "cni", "flannel", "cali", "vnet", "tunl", "kube-",
        "vxlan", "genev", "podman", "lxc",
    ];
    !SKIP.iter().any(|p| name == *p || name.starts_with(p))
}

#[derive(Debug, Clone)]
struct PrevCounters {
    uptime: Option<f64>,
    at: std::time::Instant,
    cpu_total: CpuCounters,
    cpu_cores: Vec<CpuCounters>,
    net: HashMap<String, NetDevCounters>,
    disks: HashMap<String, DiskStatCounters>,
    procs: HashMap<u32, u64>,
}

/// Per-host state that turns successive raw samples into [`HostMetrics`].
/// Slow sections (ports, containers, …) are carried over between the ticks
/// on which they are collected.
#[derive(Debug, Default)]
pub struct HostModel {
    prev: Option<PrevCounters>,
    last: HostMetrics,
}

impl HostModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last(&self) -> &HostMetrics {
        &self.last
    }

    /// Whether at least one sample has been folded in, so the next one
    /// yields rates.
    pub fn has_baseline(&self) -> bool {
        self.prev.is_some()
    }

    pub fn update(&mut self, raw: &RawSample, ts: DateTime<Utc>) -> HostMetrics {
        self.update_at(raw, ts, std::time::Instant::now())
    }

    pub fn update_at(
        &mut self,
        raw: &RawSample,
        ts: DateTime<Utc>,
        now: std::time::Instant,
    ) -> HostMetrics {
        let mut m = std::mem::take(&mut self.last);
        m.ts = ts;
        if let Some(id) = &raw.identity {
            m.hostname = id.hostname.clone();
            m.kernel = id.kernel.clone();
            m.arch = id.arch.clone();
        }
        let page_size = raw.identity.as_ref().map(|i| i.page_size).unwrap_or(4096);
        let clk_tck = raw.identity.as_ref().map(|i| i.clk_tck).unwrap_or(100) as f64;
        if let Some(os) = &raw.os {
            m.os = Some(os.pretty_name.clone());
            m.os_id = Some(os.id.clone());
        }
        if let Some(model) = &raw.cpu_model {
            m.cpu_model = model.clone();
        }
        if let Some(ips) = &raw.ips {
            m.ips = ips.clone();
        }
        m.uptime_secs = raw.uptime;

        let prev = self.prev.as_ref();
        let elapsed = prev.and_then(|p| {
            let by_uptime = match (p.uptime, raw.uptime) {
                (Some(a), Some(b)) if b > a => Some(b - a),
                _ => None,
            };
            by_uptime.or_else(|| {
                let d = now.duration_since(p.at).as_secs_f64();
                (d > 0.0).then_some(d)
            })
        });

        // CPU
        m.cpu = None;
        if let Some(stat) = &raw.stat {
            m.cores = stat.cores.len().max(1);
            if let Some(p) = prev
                && let Some((total, user, system, iowait, steal)) =
                    cpu_delta(&p.cpu_total, &stat.total)
            {
                let cores = stat
                    .cores
                    .iter()
                    .zip(p.cpu_cores.iter())
                    .map(|(c, pc)| cpu_delta(pc, c).map(|d| d.0).unwrap_or(0.0))
                    .collect();
                m.cpu = Some(CpuUsage {
                    total_pct: total,
                    user_pct: user,
                    system_pct: system,
                    iowait_pct: iowait,
                    steal_pct: steal,
                    cores,
                });
            }
        }

        // Memory
        m.mem = raw.mem.map(|mem| {
            let used = mem.total.saturating_sub(mem.available);
            let swap_used = mem.swap_total.saturating_sub(mem.swap_free);
            MemUsage {
                total: mem.total,
                used,
                available: mem.available,
                cache: mem.buffers + mem.cached + mem.sreclaimable,
                used_pct: pct(used as f64, mem.total as f64),
                swap_total: mem.swap_total,
                swap_used,
                swap_pct: pct(swap_used as f64, mem.swap_total as f64),
            }
        });

        m.load = raw.load;
        m.load_per_core = raw.load.map(|l| l.one / m.cores.max(1) as f64);

        // Network
        let prev_net = prev.map(|p| &p.net);
        m.net = raw
            .net
            .iter()
            .filter(|n| n.name != "lo" && !n.name.starts_with("veth"))
            .map(|n| {
                let (rx_bps, tx_bps) = match (prev_net.and_then(|pn| pn.get(&n.name)), elapsed) {
                    (Some(p), Some(dt)) if n.rx_bytes >= p.rx_bytes && n.tx_bytes >= p.tx_bytes => {
                        (
                            (n.rx_bytes - p.rx_bytes) as f64 / dt,
                            (n.tx_bytes - p.tx_bytes) as f64 / dt,
                        )
                    }
                    _ => (0.0, 0.0),
                };
                NetRate {
                    iface: n.name.clone(),
                    rx_bps,
                    tx_bps,
                    rx_total: n.rx_bytes,
                    tx_total: n.tx_bytes,
                    errors: n.rx_errs + n.tx_errs,
                    drops: n.rx_drop + n.tx_drop,
                }
            })
            .collect();
        if prev.is_some() && !raw.net.is_empty() {
            let primary = m.net.iter().filter(|n| is_primary_iface(&n.iface));
            let (rx, tx) = primary.fold((0.0, 0.0), |(r, t), n| (r + n.rx_bps, t + n.tx_bps));
            m.net_rx_bps = Some(rx);
            m.net_tx_bps = Some(tx);
        } else {
            m.net_rx_bps = None;
            m.net_tx_bps = None;
        }

        // Disks
        if !raw.df.is_empty() {
            m.disks = disk_usage(&raw.df);
        }
        let prev_disks = prev.map(|p| &p.disks);
        m.disk_io = physical_disks(&raw.diskstats)
            .into_iter()
            .map(|d| {
                let mut r = DiskIoRate {
                    device: d.name.clone(),
                    ..Default::default()
                };
                if let (Some(p), Some(dt)) = (prev_disks.and_then(|pd| pd.get(&d.name)), elapsed)
                    && d.sectors_read >= p.sectors_read
                    && d.sectors_written >= p.sectors_written
                {
                    r.read_bps = (d.sectors_read - p.sectors_read) as f64 * 512.0 / dt;
                    r.write_bps = (d.sectors_written - p.sectors_written) as f64 * 512.0 / dt;
                    r.read_iops = d.reads.saturating_sub(p.reads) as f64 / dt;
                    r.write_iops = d.writes.saturating_sub(p.writes) as f64 / dt;
                    r.util_pct = pct(d.io_ms.saturating_sub(p.io_ms) as f64, dt * 1000.0);
                }
                r
            })
            .collect();

        // Processes
        let prev_procs = prev.map(|p| &p.procs);
        let mem_total = raw.mem.map(|m| m.total).unwrap_or(0) as f64;
        if !raw.procs.is_empty() {
            let mut procs: Vec<ProcInfo> = raw
                .procs
                .iter()
                .map(|p| {
                    let ticks = p.utime + p.stime;
                    let cpu_pct = match (prev_procs.and_then(|pp| pp.get(&p.pid)), elapsed) {
                        (Some(&prev_ticks), Some(dt)) if ticks >= prev_ticks => {
                            Some((ticks - prev_ticks) as f64 / clk_tck / dt * 100.0)
                        }
                        _ => None,
                    };
                    let rss = p.rss_pages * page_size;
                    ProcInfo {
                        pid: p.pid,
                        user: raw.users.get(&p.pid).cloned().unwrap_or_default(),
                        name: p.comm.clone(),
                        state: p.state.clone(),
                        cpu_pct,
                        mem_pct: pct(rss as f64, mem_total),
                        rss,
                    }
                })
                .collect();
            m.proc_count = procs.len();
            m.procs = top_processes(&mut procs, 12);
        }

        if let Some(p) = &raw.ports {
            m.ports = p.clone();
        }
        if let Some(c) = &raw.containers {
            m.containers = c.clone();
        }
        if let Some(u) = &raw.failed_units {
            m.failed_units = u.clone();
        }
        if let Some(l) = &raw.failed_logins {
            m.failed_logins = l.clone();
        }
        if let Some(u) = &raw.updates {
            m.updates = u.clone();
        }

        if let Some(stat) = &raw.stat {
            self.prev = Some(PrevCounters {
                uptime: raw.uptime,
                at: now,
                cpu_total: stat.total,
                cpu_cores: stat.cores.clone(),
                net: raw
                    .net
                    .iter()
                    .map(|n| (n.name.clone(), n.clone()))
                    .collect(),
                disks: raw
                    .diskstats
                    .iter()
                    .map(|d| (d.name.clone(), d.clone()))
                    .collect(),
                procs: raw
                    .procs
                    .iter()
                    .map(|p| (p.pid, p.utime + p.stime))
                    .collect(),
            });
        }
        self.last = m.clone();
        m
    }
}

/// The union of the busiest processes by CPU and by memory.
fn top_processes(procs: &mut [ProcInfo], n: usize) -> Vec<ProcInfo> {
    procs.sort_by(|a, b| {
        b.cpu_pct
            .unwrap_or(0.0)
            .total_cmp(&a.cpu_pct.unwrap_or(0.0))
            .then(b.rss.cmp(&a.rss))
    });
    let mut out: Vec<ProcInfo> = procs.iter().take(n).cloned().collect();
    procs.sort_by_key(|p| std::cmp::Reverse(p.rss));
    for p in procs.iter().take(n) {
        if !out.iter().any(|o| o.pid == p.pid) {
            out.push(p.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::parse::*;
    use crate::collect::{CpuStat, MemInfo, ProcStat};
    use std::time::{Duration, Instant};

    fn raw(cpu: [u64; 4], uptime: f64, rx: u64, sectors: u64, proc_ticks: u64) -> RawSample {
        let c = CpuCounters {
            user: cpu[0],
            system: cpu[1],
            idle: cpu[2],
            iowait: cpu[3],
            ..Default::default()
        };
        RawSample {
            stat: Some(CpuStat {
                total: c,
                cores: vec![c],
                ..Default::default()
            }),
            mem: Some(MemInfo {
                total: 1000 * 4096,
                available: 750 * 4096,
                swap_total: 100,
                swap_free: 50,
                ..Default::default()
            }),
            uptime: Some(uptime),
            net: vec![NetDevCounters {
                name: "eth0".into(),
                rx_bytes: rx,
                tx_bytes: rx / 2,
                ..Default::default()
            }],
            diskstats: vec![
                DiskStatCounters {
                    name: "sda".into(),
                    reads: sectors / 8,
                    sectors_read: sectors,
                    writes: 1,
                    io_ms: sectors / 10,
                    ..Default::default()
                },
                DiskStatCounters {
                    name: "sda1".into(),
                    reads: 1,
                    ..Default::default()
                },
                DiskStatCounters {
                    name: "loop0".into(),
                    reads: 5,
                    ..Default::default()
                },
            ],
            procs: vec![ProcStat {
                pid: 7,
                state: "R".into(),
                utime: proc_ticks,
                stime: 0,
                rss_pages: 100,
                comm: "worker".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn first_sample_has_no_rates() {
        let mut model = HostModel::new();
        let m = model.update(&raw([100, 50, 800, 50], 10.0, 1000, 0, 0), Utc::now());
        assert!(m.cpu.is_none());
        assert_eq!(m.net_rx_bps, None);
        assert_eq!(m.procs[0].cpu_pct, None);
        let mem = m.mem.unwrap();
        assert_eq!(mem.used_pct, 25.0);
        assert_eq!(mem.swap_pct, 50.0);
    }

    #[test]
    fn rates_from_deltas() {
        let mut model = HostModel::new();
        let t0 = Instant::now();
        model.update_at(
            &raw([100, 50, 800, 50], 10.0, 1000, 1000, 0),
            Utc::now(),
            t0,
        );
        // +100 jiffies: 30 user, 10 system, 50 idle, 10 iowait over 2 s.
        let m = model.update_at(
            &raw([130, 60, 850, 60], 12.0, 5000, 3048, 100),
            Utc::now(),
            t0 + Duration::from_secs(2),
        );
        let cpu = m.cpu.unwrap();
        assert!((cpu.total_pct - 40.0).abs() < 1e-9);
        assert!((cpu.user_pct - 30.0).abs() < 1e-9);
        assert!((cpu.iowait_pct - 10.0).abs() < 1e-9);
        assert_eq!(cpu.cores.len(), 1);
        assert_eq!(m.net_rx_bps, Some(2000.0));
        assert_eq!(m.net_tx_bps, Some(1000.0));
        assert_eq!(m.disk_io.len(), 1, "partitions and loop devices are hidden");
        let io = &m.disk_io[0];
        assert_eq!(io.read_bps, 2048.0 * 512.0 / 2.0);
        // 100 ticks at 100 Hz over 2 s = 50% of a core.
        assert_eq!(m.procs[0].cpu_pct, Some(50.0));
        assert!((m.procs[0].mem_pct - 10.0).abs() < 1e-9);
    }

    #[test]
    fn counter_reset_yields_zero_rate() {
        let mut model = HostModel::new();
        model.update(&raw([100, 50, 800, 50], 10.0, 9000, 1000, 0), Utc::now());
        let m = model.update(&raw([130, 60, 850, 60], 12.0, 10, 500, 0), Utc::now());
        assert_eq!(m.net[0].rx_bps, 0.0);
        assert_eq!(m.disk_io[0].read_bps, 0.0);
    }

    #[test]
    fn slow_sections_carry_over() {
        let mut model = HostModel::new();
        let mut r = raw([1, 1, 1, 1], 1.0, 0, 0, 0);
        r.ports = Some(Probe::Ok(vec![ListenPort {
            port: 22,
            addr: "0.0.0.0".into(),
        }]));
        model.update(&r, Utc::now());
        let m = model.update(&raw([2, 2, 2, 2], 2.0, 0, 0, 0), Utc::now());
        assert_eq!(m.ports.ok().unwrap().len(), 1);
        assert!(m.containers.is_pending());
    }

    #[test]
    fn disk_filtering_on_fixture() {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/alpine/full.txt"
        ));
        let s = split_sections(text, "0123456789abcdef");
        let df = parse_df(s.get("df").unwrap());
        let disks = disk_usage(&df);
        let mounts: Vec<&str> = disks.iter().map(|d| d.mount.as_str()).collect();
        assert!(mounts.contains(&"/"));
        assert!(
            !mounts
                .iter()
                .any(|m| m.starts_with("/proc") || m.starts_with("/dev"))
        );
        // /etc/hosts etc. are bind mounts of one device: only one survives.
        assert_eq!(mounts.iter().filter(|m| m.starts_with("/etc")).count(), 1);
    }

    #[test]
    fn partitions() {
        let all: Vec<DiskStatCounters> = [
            "sda",
            "sda1",
            "nvme0n1",
            "nvme0n1p2",
            "mmcblk0",
            "mmcblk0p1",
            "md0",
            "dm-0",
        ]
        .iter()
        .map(|n| DiskStatCounters {
            name: n.to_string(),
            reads: 1,
            ..Default::default()
        })
        .collect();
        let names: Vec<&str> = physical_disks(&all)
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, vec!["sda", "nvme0n1", "mmcblk0", "md0", "dm-0"]);
    }

    #[test]
    fn primary_interfaces() {
        assert!(is_primary_iface("eth0"));
        assert!(is_primary_iface("ens3"));
        assert!(!is_primary_iface("docker0"));
        assert!(!is_primary_iface("veth12ab"));
        assert!(!is_primary_iface("lo"));
    }
}
