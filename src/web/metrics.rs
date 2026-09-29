//! Prometheus text exposition of the fleet state.

use std::fmt::Write as _;

use crate::collect::Probe;
use crate::engine::{ConnState, FleetState};

fn esc(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

struct Family {
    name: &'static str,
    help: &'static str,
    kind: &'static str,
    samples: Vec<(String, f64)>,
}

impl Family {
    fn new(name: &'static str, kind: &'static str, help: &'static str) -> Self {
        Family {
            name,
            help,
            kind,
            samples: Vec::new(),
        }
    }

    fn push(&mut self, labels: &[(&str, &str)], v: f64) {
        if !v.is_finite() {
            return;
        }
        let l = labels
            .iter()
            .map(|(k, v)| format!("{k}=\"{}\"", esc(v)))
            .collect::<Vec<_>>()
            .join(",");
        self.samples.push((l, v));
    }

    fn write(&self, out: &mut String) {
        if self.samples.is_empty() {
            return;
        }
        let _ = writeln!(out, "# HELP {} {}", self.name, self.help);
        let _ = writeln!(out, "# TYPE {} {}", self.name, self.kind);
        for (l, v) in &self.samples {
            let _ = writeln!(out, "{}{{{}}} {}", self.name, l, v);
        }
    }
}

/// Renders the fleet in the Prometheus text format (version 0.0.4).
pub fn render(fleet: &FleetState) -> String {
    let mut up = Family::new(
        "skry_up",
        "gauge",
        "Whether skry can reach and read the host (1) or not (0).",
    );
    let mut status = Family::new(
        "skry_status",
        "gauge",
        "Host status: 0 ok, 1 baseline deviation, 2 warning, 3 critical, 4 unreachable.",
    );
    let mut cpu = Family::new("skry_cpu_usage_percent", "gauge", "CPU busy percentage.");
    let mut core = Family::new(
        "skry_cpu_core_usage_percent",
        "gauge",
        "CPU busy percentage per core.",
    );
    let mut cores = Family::new("skry_cpu_cores", "gauge", "Number of CPU cores.");
    let mut mem_used = Family::new(
        "skry_memory_used_bytes",
        "gauge",
        "Memory in use (total minus available).",
    );
    let mut mem_total = Family::new("skry_memory_total_bytes", "gauge", "Total memory.");
    let mut mem_pct = Family::new(
        "skry_memory_usage_percent",
        "gauge",
        "Memory usage percentage.",
    );
    let mut swap_used = Family::new("skry_swap_used_bytes", "gauge", "Swap in use.");
    let mut swap_total = Family::new("skry_swap_total_bytes", "gauge", "Total swap.");
    let mut load1 = Family::new("skry_load1", "gauge", "One-minute load average.");
    let mut load5 = Family::new("skry_load5", "gauge", "Five-minute load average.");
    let mut load15 = Family::new("skry_load15", "gauge", "Fifteen-minute load average.");
    let mut uptime = Family::new("skry_uptime_seconds", "gauge", "Seconds since boot.");
    let mut rx = Family::new(
        "skry_network_receive_bytes_per_second",
        "gauge",
        "Receive rate per interface.",
    );
    let mut tx = Family::new(
        "skry_network_transmit_bytes_per_second",
        "gauge",
        "Transmit rate per interface.",
    );
    let mut rx_total = Family::new(
        "skry_network_receive_bytes_total",
        "counter",
        "Bytes received per interface since boot.",
    );
    let mut tx_total = Family::new(
        "skry_network_transmit_bytes_total",
        "counter",
        "Bytes transmitted per interface since boot.",
    );
    let mut fs_used = Family::new(
        "skry_filesystem_used_bytes",
        "gauge",
        "Used bytes per filesystem.",
    );
    let mut fs_size = Family::new(
        "skry_filesystem_size_bytes",
        "gauge",
        "Size per filesystem.",
    );
    let mut fs_pct = Family::new(
        "skry_filesystem_usage_percent",
        "gauge",
        "Usage percentage per filesystem.",
    );
    let mut rd = Family::new(
        "skry_disk_read_bytes_per_second",
        "gauge",
        "Read rate per block device.",
    );
    let mut wr = Family::new(
        "skry_disk_write_bytes_per_second",
        "gauge",
        "Write rate per block device.",
    );
    let mut util = Family::new(
        "skry_disk_utilization_percent",
        "gauge",
        "Time the device was busy, percent.",
    );
    let mut procs = Family::new("skry_processes", "gauge", "Number of processes.");
    let mut units = Family::new("skry_failed_units", "gauge", "Failed systemd units.");
    let mut logins = Family::new(
        "skry_failed_ssh_logins_24h",
        "gauge",
        "Failed SSH authentication events in the last 24 hours.",
    );
    let mut updates = Family::new(
        "skry_pending_updates",
        "gauge",
        "Pending package updates from cached metadata.",
    );
    let mut sec_updates = Family::new(
        "skry_pending_security_updates",
        "gauge",
        "Pending security updates from cached metadata.",
    );
    let mut ports = Family::new("skry_listening_ports", "gauge", "Listening TCP sockets.");
    let mut findings = Family::new(
        "skry_security_findings",
        "gauge",
        "Security pulse findings.",
    );
    let mut containers = Family::new("skry_containers", "gauge", "Containers by state.");
    let mut deviations = Family::new(
        "skry_baseline_deviations",
        "gauge",
        "Metrics currently deviating from their baseline.",
    );
    let mut tls_days = Family::new(
        "skry_tls_certificate_days_left",
        "gauge",
        "Days until the TLS certificate expires.",
    );
    let mut tls_valid = Family::new(
        "skry_tls_certificate_valid",
        "gauge",
        "Whether the TLS certificate chain validates (1) or not (0).",
    );

    for h in &fleet.hosts {
        let host = h.name.as_str();
        let hl = [("host", host)];
        up.push(
            &hl,
            if h.conn == ConnState::Failed {
                0.0
            } else {
                1.0
            },
        );
        status.push(&hl, h.status.rank() as f64);
        deviations.push(&hl, h.deviations.len() as f64);
        findings.push(&hl, h.findings.len() as f64);
        let Some(m) = &h.metrics else { continue };
        if h.conn == ConnState::Failed {
            continue;
        }
        cores.push(&hl, m.cores as f64);
        if let Some(c) = &m.cpu {
            cpu.push(&hl, c.total_pct);
            for (i, v) in c.cores.iter().enumerate() {
                core.push(&[("host", host), ("core", &i.to_string())], *v);
            }
        }
        if let Some(x) = &m.mem {
            mem_used.push(&hl, x.used as f64);
            mem_total.push(&hl, x.total as f64);
            mem_pct.push(&hl, x.used_pct);
            swap_used.push(&hl, x.swap_used as f64);
            swap_total.push(&hl, x.swap_total as f64);
        }
        if let Some(l) = &m.load {
            load1.push(&hl, l.one);
            load5.push(&hl, l.five);
            load15.push(&hl, l.fifteen);
        }
        if let Some(u) = m.uptime_secs {
            uptime.push(&hl, u);
        }
        for n in &m.net {
            let l = [("host", host), ("interface", n.iface.as_str())];
            rx.push(&l, n.rx_bps);
            tx.push(&l, n.tx_bps);
            rx_total.push(&l, n.rx_total as f64);
            tx_total.push(&l, n.tx_total as f64);
        }
        for d in &m.disks {
            let l = [
                ("host", host),
                ("mount", d.mount.as_str()),
                ("device", d.filesystem.as_str()),
            ];
            fs_used.push(&l, d.used as f64);
            fs_size.push(&l, d.total as f64);
            fs_pct.push(&l, d.used_pct);
        }
        for d in &m.disk_io {
            let l = [("host", host), ("device", d.device.as_str())];
            rd.push(&l, d.read_bps);
            wr.push(&l, d.write_bps);
            util.push(&l, d.util_pct);
        }
        procs.push(&hl, m.proc_count as f64);
        if let Probe::Ok(u) = &m.failed_units {
            units.push(&hl, u.len() as f64);
        }
        if let Probe::Ok(l) = &m.failed_logins {
            logins.push(&hl, l.total as f64);
        }
        if let Probe::Ok(u) = &m.updates {
            updates.push(&hl, u.total as f64);
            if let Some(s) = u.security {
                sec_updates.push(&hl, s as f64);
            }
        }
        if let Probe::Ok(p) = &m.ports {
            ports.push(&hl, p.len() as f64);
        }
        if let Probe::Ok(c) = &m.containers {
            let mut by_state: std::collections::BTreeMap<&str, usize> = Default::default();
            for x in c {
                *by_state.entry(x.state.as_str()).or_default() += 1;
            }
            for (state, n) in by_state {
                containers.push(&[("host", host), ("state", state)], n as f64);
            }
        }
    }
    for t in &fleet.tls {
        let l = [("endpoint", t.endpoint.as_str())];
        if let Some(d) = t.days_left {
            tls_days.push(&l, d as f64);
        }
        tls_valid.push(&l, if t.valid { 1.0 } else { 0.0 });
    }

    let mut out = String::with_capacity(16 * 1024);
    for f in [
        &up,
        &status,
        &cpu,
        &core,
        &cores,
        &mem_used,
        &mem_total,
        &mem_pct,
        &swap_used,
        &swap_total,
        &load1,
        &load5,
        &load15,
        &uptime,
        &rx,
        &tx,
        &rx_total,
        &tx_total,
        &fs_used,
        &fs_size,
        &fs_pct,
        &rd,
        &wr,
        &util,
        &procs,
        &units,
        &logins,
        &updates,
        &sec_updates,
        &ports,
        &findings,
        &containers,
        &deviations,
        &tls_days,
        &tls_valid,
    ] {
        f.write(&mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposition_of_demo_fleet() {
        let now = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 9, 29, 12, 0, 0).unwrap();
        let text = render(&crate::tui::demo::fleet(now, 0.0));
        insta::assert_snapshot!(text);
    }

    #[test]
    fn label_escaping() {
        assert_eq!(esc("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn every_line_is_well_formed() {
        let now = chrono::Utc::now();
        let text = render(&crate::tui::demo::fleet(now, 1.0));
        for line in text.lines() {
            if line.starts_with('#') {
                assert!(line.starts_with("# HELP skry_") || line.starts_with("# TYPE skry_"));
                continue;
            }
            let (series, value) = line.rsplit_once(' ').unwrap();
            assert!(
                series.starts_with("skry_") && series.ends_with('}'),
                "{line}"
            );
            value.parse::<f64>().unwrap();
        }
    }
}
