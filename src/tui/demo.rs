//! A synthetic fleet for UI tests and the `skry demo` command (used to
//! record the README animation without real servers).

use chrono::{DateTime, Duration, Utc};

use crate::baseline::{Deviation, Metric};
use crate::collect::{
    Container, FailedLogins, FailedUnit, IpAddrEntry, ListenPort, LoadAvg, Probe, Updates,
};
use crate::config::{SecurityConfig, Target};
use crate::engine::{ConnState, FleetState, HostError, HostState};
use crate::model::{
    CpuUsage, DiskIoRate, DiskUsage, Health, HostMetrics, MemUsage, NetRate, ProcInfo, Thresholds,
};
use crate::security::{TlsResult, host_findings};
use crate::ssh::FailureKind;

const GIB: u64 = 1024 * 1024 * 1024;

struct Spec {
    name: &'static str,
    group: &'static str,
    os: &'static str,
    cpu: f64,
    mem: f64,
    disk: f64,
    load: f64,
    cores: usize,
}

fn wave(seed: usize, t: f64, amp: f64) -> f64 {
    ((t / 7.0 + seed as f64 * 1.7).sin() * 0.6 + (t / 3.0 + seed as f64).cos() * 0.4) * amp
}

fn metrics(s: &Spec, seed: usize, ts: DateTime<Utc>, t: f64) -> HostMetrics {
    let cpu = (s.cpu + wave(seed, t, 6.0)).clamp(0.5, 100.0);
    let cores: Vec<f64> = (0..s.cores)
        .map(|i| (cpu + wave(seed + i * 3, t, 15.0)).clamp(0.0, 100.0))
        .collect();
    let total = 16 * GIB;
    let used = (total as f64 * s.mem / 100.0) as u64;
    let load = (s.load + wave(seed, t, 0.2)).max(0.0);
    HostMetrics {
        ts,
        hostname: s.name.to_string(),
        os: Some(s.os.to_string()),
        os_id: Some(s.os.split_whitespace().next().unwrap_or("").to_lowercase()),
        kernel: "6.1.0-25-amd64".into(),
        arch: "x86_64".into(),
        cpu_model: Some("AMD EPYC 7B13".into()),
        cores: s.cores,
        uptime_secs: Some(86400.0 * 23.0 + seed as f64 * 3600.0),
        cpu: Some(CpuUsage {
            total_pct: cpu,
            user_pct: cpu * 0.7,
            system_pct: cpu * 0.25,
            iowait_pct: cpu * 0.05,
            steal_pct: 0.0,
            cores,
        }),
        mem: Some(MemUsage {
            total,
            used,
            available: total - used,
            cache: 3 * GIB,
            used_pct: s.mem,
            swap_total: 2 * GIB,
            swap_used: GIB / 8,
            swap_pct: 6.25,
        }),
        load: Some(LoadAvg {
            one: load * s.cores as f64,
            five: load * s.cores as f64 * 0.9,
            fifteen: load * s.cores as f64 * 0.8,
            running: 2,
            total: 312,
        }),
        load_per_core: Some(load),
        net: vec![NetRate {
            iface: "eth0".into(),
            rx_bps: 1_250_000.0 + wave(seed, t, 400_000.0),
            tx_bps: 420_000.0 + wave(seed + 1, t, 100_000.0),
            rx_total: 912 * GIB,
            tx_total: 301 * GIB,
            errors: 0,
            drops: 0,
        }],
        net_rx_bps: Some(1_250_000.0 + wave(seed, t, 400_000.0)),
        net_tx_bps: Some(420_000.0 + wave(seed + 1, t, 100_000.0)),
        disks: vec![
            DiskUsage {
                mount: "/".into(),
                filesystem: "/dev/sda1".into(),
                total: 80 * GIB,
                used: (80.0 * s.disk / 100.0) as u64 * GIB,
                avail: (80.0 * (100.0 - s.disk) / 100.0) as u64 * GIB,
                used_pct: s.disk,
            },
            DiskUsage {
                mount: "/var/lib/data".into(),
                filesystem: "/dev/sdb1".into(),
                total: 500 * GIB,
                used: 210 * GIB,
                avail: 290 * GIB,
                used_pct: 42.0,
            },
        ],
        disk_io: vec![DiskIoRate {
            device: "sda".into(),
            read_bps: 2_400_000.0,
            write_bps: 5_100_000.0,
            read_iops: 120.0,
            write_iops: 340.0,
            util_pct: 18.0,
        }],
        ips: vec![IpAddrEntry {
            iface: "eth0".into(),
            family: "inet".into(),
            cidr: format!("10.0.1.{}/24", 10 + seed),
            scope: "global".into(),
        }],
        procs: vec![
            proc(1234, "www-data", "nginx", cpu * 0.4, 2.1, 350),
            proc(2210, "postgres", "postgres", cpu * 0.3, 18.4, 3000),
            proc(981, "root", "dockerd", 1.2, 1.4, 230),
            proc(1, "root", "systemd", 0.0, 0.1, 12),
        ],
        proc_count: 212,
        ports: Probe::Ok(vec![
            ListenPort {
                port: 22,
                addr: "0.0.0.0".into(),
            },
            ListenPort {
                port: 443,
                addr: "0.0.0.0".into(),
            },
            ListenPort {
                port: 5432,
                addr: "127.0.0.1".into(),
            },
        ]),
        containers: Probe::Ok(vec![Container {
            runtime: "docker".into(),
            name: "api".into(),
            image: "ghcr.io/example/api:1.42".into(),
            status: "Up 3 days".into(),
            state: "running".into(),
            cpu_pct: Some(4.2),
            mem_usage: Some("312MiB / 15.6GiB".into()),
            mem_pct: Some(1.95),
        }]),
        failed_units: Probe::Ok(vec![]),
        failed_logins: Probe::Ok(FailedLogins {
            source: "journal".into(),
            total: 3,
            top_sources: vec![("203.0.113.50".into(), 3)],
        }),
        updates: Probe::Ok(Updates {
            manager: "apt".into(),
            total: 4,
            security: Some(0),
            security_packages: vec![],
        }),
    }
}

fn proc(pid: u32, user: &str, name: &str, cpu: f64, mem: f64, rss_mib: u64) -> ProcInfo {
    ProcInfo {
        pid,
        user: user.into(),
        name: name.into(),
        state: "S".into(),
        cpu_pct: Some(cpu),
        mem_pct: mem,
        rss: rss_mib * 1024 * 1024,
    }
}

/// Demo fleet at time `now`; `t` animates values (seconds since start).
pub fn fleet(now: DateTime<Utc>, t: f64) -> FleetState {
    let specs = [
        Spec {
            name: "web-1",
            group: "production",
            os: "Debian GNU/Linux 12 (bookworm)",
            cpu: 23.0,
            mem: 41.0,
            disk: 55.0,
            load: 0.3,
            cores: 4,
        },
        Spec {
            name: "web-2",
            group: "production",
            os: "Debian GNU/Linux 12 (bookworm)",
            cpu: 31.0,
            mem: 44.0,
            disk: 57.0,
            load: 0.4,
            cores: 4,
        },
        Spec {
            name: "db-1",
            group: "production",
            os: "Rocky Linux 9.4 (Blue Onyx)",
            cpu: 96.0,
            mem: 88.0,
            disk: 71.0,
            load: 3.4,
            cores: 8,
        },
        Spec {
            name: "cache-1",
            group: "production",
            os: "Alpine Linux v3.20",
            cpu: 12.0,
            mem: 62.0,
            disk: 20.0,
            load: 0.2,
            cores: 2,
        },
        Spec {
            name: "ci-runner",
            group: "build",
            os: "Ubuntu 24.04.1 LTS",
            cpu: 58.0,
            mem: 67.0,
            disk: 84.0,
            load: 1.1,
            cores: 16,
        },
        Spec {
            name: "backup",
            group: "ops",
            os: "Debian GNU/Linux 12 (bookworm)",
            cpu: 4.0,
            mem: 18.0,
            disk: 63.0,
            load: 0.05,
            cores: 2,
        },
    ];
    let thresholds = Thresholds::default();
    let security = SecurityConfig::default();
    let mut hosts: Vec<HostState> = specs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let target = Target {
                spec: s.name.into(),
                groups: vec![s.group.into()],
                allowed_ports: Some([22, 443].into()),
            };
            let mut h = HostState::new(&target, Some(format!("deploy@10.0.1.{}:22", 10 + i)));
            let mut m = metrics(s, i, now, t);
            if s.name == "cache-1" {
                h.deviations = vec![Deviation {
                    metric: Metric::Memory,
                    value: 62.0,
                    mean: 38.0,
                    std: 4.1,
                    z: 5.9,
                }];
            }
            if s.name == "db-1" {
                m.failed_units = Probe::Ok(vec![FailedUnit {
                    unit: "pg-backup.service".into(),
                    load: "loaded".into(),
                    active: "failed".into(),
                    sub: "failed".into(),
                    description: "Nightly PostgreSQL backup".into(),
                }]);
                m.failed_logins = Probe::Ok(FailedLogins {
                    source: "/var/log/secure".into(),
                    total: 1840,
                    top_sources: vec![("198.51.100.23".into(), 1702), ("203.0.113.9".into(), 138)],
                });
                m.updates = Probe::Ok(Updates {
                    manager: "dnf".into(),
                    total: 23,
                    security: Some(5),
                    security_packages: vec!["openssl-libs".into(), "curl".into(), "kernel".into()],
                });
                m.ports = Probe::Ok(vec![
                    ListenPort {
                        port: 22,
                        addr: "0.0.0.0".into(),
                    },
                    ListenPort {
                        port: 5432,
                        addr: "0.0.0.0".into(),
                    },
                ]);
            }
            h.health = Health::evaluate(&m, &thresholds);
            let allowed = h
                .allowed_ports
                .as_ref()
                .map(|p| p.iter().copied().collect());
            h.findings = host_findings(&m, allowed.as_ref(), &security);
            h.metrics = Some(m);
            h.conn = ConnState::Connected;
            h.last_update = Some(now);
            h.refresh_status();
            h
        })
        .collect();
    let mut down = HostState::new(
        &Target {
            spec: "legacy-app".into(),
            groups: vec!["ops".into()],
            allowed_ports: None,
        },
        Some("admin@10.0.9.4:22".into()),
    );
    down.conn = ConnState::Failed;
    down.error = Some(HostError {
        kind: FailureKind::Auth,
        message: "authentication failed for admin@10.0.9.4:22 (tried 2 keys)".into(),
        since: now - Duration::minutes(12),
    });
    down.refresh_status();
    hosts.push(down);
    let mut keyhost = HostState::new(
        &Target {
            spec: "staging-1".into(),
            groups: vec!["staging".into()],
            allowed_ports: None,
        },
        Some("deploy@staging-1.example.com:22".into()),
    );
    keyhost.conn = ConnState::Failed;
    keyhost.error = Some(HostError {
        kind: FailureKind::HostKey,
        message: "unknown host key for staging-1.example.com (SHA256:Zm9vYmFy…); verify it, then rerun with --accept-new".into(),
        since: now - Duration::minutes(2),
    });
    keyhost.refresh_status();
    hosts.push(keyhost);

    FleetState {
        hosts,
        tls: vec![
            TlsResult {
                endpoint: "www.example.com:443".into(),
                not_after: Some(now + Duration::days(64)),
                days_left: Some(64),
                subject: Some("www.example.com".into()),
                issuer: Some("R11".into()),
                valid: true,
                problem: None,
                level: crate::model::Level::Ok,
            },
            TlsResult {
                endpoint: "api.example.com:443".into(),
                not_after: Some(now + Duration::days(5)),
                days_left: Some(5),
                subject: Some("api.example.com".into()),
                issuer: Some("R10".into()),
                valid: true,
                problem: None,
                level: crate::model::Level::Critical,
            },
        ],
        started: now - Duration::hours(2),
        generation: 1,
    }
}
