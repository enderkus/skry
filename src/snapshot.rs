//! Incident snapshots: the full current state of selected hosts as a
//! Markdown report plus a JSON file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::collect::Probe;
use crate::config::{Webhook, WebhookKind};
use crate::engine::{FleetState, HostState};
use crate::security::TlsResult;
use crate::util::{human_bytes, human_duration, pct};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub taken_at: DateTime<Utc>,
    pub skry_version: String,
    pub hosts: Vec<HostState>,
    pub tls: Vec<TlsResult>,
}

fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

impl Snapshot {
    pub fn new(hosts: Vec<HostState>, tls: Vec<TlsResult>, taken_at: DateTime<Utc>) -> Self {
        Snapshot {
            taken_at,
            skry_version: env!("CARGO_PKG_VERSION").to_string(),
            hosts,
            tls,
        }
    }

    /// Snapshot of the whole fleet, or of the named hosts only.
    pub fn from_fleet(fleet: &FleetState, only: Option<&[String]>) -> Self {
        let hosts = fleet
            .hosts
            .iter()
            .filter(|h| only.is_none_or(|o| o.contains(&h.name)))
            .cloned()
            .collect();
        Snapshot::new(hosts, fleet.tls.clone(), Utc::now())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn file_stem(&self) -> String {
        format!("skry-snapshot-{}", self.taken_at.format("%Y%m%d-%H%M%S"))
    }

    /// Writes `<stem>.md` and `<stem>.json` into `dir`.
    pub fn write(&self, dir: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
        std::fs::create_dir_all(dir)?;
        let md = dir.join(format!("{}.md", self.file_stem()));
        let json = dir.join(format!("{}.json", self.file_stem()));
        std::fs::write(&md, self.to_markdown())?;
        std::fs::write(&json, self.to_json())?;
        Ok((md, json))
    }

    /// A short summary suitable for chat webhooks.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "skry snapshot {} — {} hosts",
            self.taken_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.hosts.len()
        );
        for h in &self.hosts {
            let _ = write!(s, "\n• {} [{}]", h.name, h.status.label());
            if let Some(m) = &h.metrics {
                let _ = write!(
                    s,
                    " cpu {} mem {} disk {}",
                    pct(m.cpu_pct()),
                    pct(m.mem_pct()),
                    pct(m.disk_max_pct())
                );
            }
            if let Some(e) = &h.error {
                let _ = write!(s, " — {}", e.message);
            }
            for b in &h.health.breaches {
                let _ = write!(s, "\n    {}", b.describe());
            }
        }
        s
    }

    pub fn to_markdown(&self) -> String {
        let mut o = String::new();
        let _ = writeln!(o, "# skry incident snapshot\n");
        let _ = writeln!(
            o,
            "Taken {} by skry {} — {} host(s).\n",
            self.taken_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.skry_version,
            self.hosts.len()
        );
        let _ = writeln!(o, "## Summary\n");
        let _ = writeln!(
            o,
            "| Host | Status | CPU | Memory | Disk | Load/core | Uptime | Notes |"
        );
        let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- | --- |");
        for h in &self.hosts {
            let m = h.metrics.as_ref();
            let notes = if let Some(e) = &h.error {
                e.message.clone()
            } else {
                let mut n: Vec<String> = h.health.breaches.iter().map(|b| b.describe()).collect();
                n.extend(h.deviations.iter().map(|d| d.describe()));
                n.join("; ")
            };
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                cell(&h.name),
                h.status.label(),
                pct(m.and_then(|m| m.cpu_pct())),
                pct(m.and_then(|m| m.mem_pct())),
                pct(m.and_then(|m| m.disk_max_pct())),
                m.and_then(|m| m.load_per_core)
                    .map(|l| format!("{l:.2}"))
                    .unwrap_or_else(|| "n/a".into()),
                m.and_then(|m| m.uptime_secs)
                    .map(human_duration)
                    .unwrap_or_else(|| "n/a".into()),
                cell(&notes)
            );
        }
        if !self.tls.is_empty() {
            let _ = writeln!(o, "\n## TLS certificates\n");
            let _ = writeln!(o, "| Endpoint | Status | Expires | Subject | Issuer |");
            let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
            for t in &self.tls {
                let _ = writeln!(
                    o,
                    "| {} | {} | {} | {} | {} |",
                    cell(&t.endpoint),
                    t.level.as_str(),
                    cell(&t.describe()),
                    cell(t.subject.as_deref().unwrap_or("")),
                    cell(t.issuer.as_deref().unwrap_or(""))
                );
            }
        }
        for h in &self.hosts {
            host_markdown(&mut o, h);
        }
        o
    }
}

fn probe_note<T>(p: &Probe<T>) -> String {
    p.describe().unwrap_or_default()
}

fn host_markdown(o: &mut String, h: &HostState) {
    let _ = writeln!(o, "\n## {}\n", h.name);
    let _ = writeln!(o, "- Status: **{}**", h.status.label());
    if let Some(a) = &h.addr {
        let _ = writeln!(o, "- Address: `{a}`");
    }
    if !h.groups.is_empty() {
        let _ = writeln!(o, "- Groups: {}", h.groups.join(", "));
    }
    if let Some(e) = &h.error {
        let _ = writeln!(
            o,
            "- Error ({}), since {}: {}",
            e.kind.label(),
            e.since.format("%H:%M:%S"),
            e.message
        );
    }
    let Some(m) = &h.metrics else {
        return;
    };
    let _ = writeln!(o, "- Hostname: `{}`", m.hostname);
    if let Some(os) = &m.os {
        let _ = writeln!(o, "- OS: {os}");
    }
    let _ = writeln!(o, "- Kernel: {} ({})", m.kernel, m.arch);
    if let Some(model) = &m.cpu_model {
        let _ = writeln!(o, "- CPU: {model}, {} cores", m.cores);
    } else {
        let _ = writeln!(o, "- CPU: {} cores", m.cores);
    }
    if let Some(u) = m.uptime_secs {
        let _ = writeln!(o, "- Uptime: {}", human_duration(u));
    }
    let ips: Vec<&str> = m
        .ips
        .iter()
        .filter(|i| i.scope != "host")
        .map(|i| i.cidr.as_str())
        .collect();
    if !ips.is_empty() {
        let _ = writeln!(o, "- Addresses: {}", ips.join(", "));
    }
    let _ = writeln!(o, "- Sampled: {}", m.ts.format("%Y-%m-%d %H:%M:%S UTC"));

    if !h.health.breaches.is_empty() || !h.deviations.is_empty() || !h.findings.is_empty() {
        let _ = writeln!(o, "\n### Problems\n");
        for b in &h.health.breaches {
            let _ = writeln!(o, "- Threshold: {}", b.describe());
        }
        for d in &h.deviations {
            let _ = writeln!(o, "- Baseline deviation: {}", d.describe());
        }
        for f in &h.findings {
            let _ = writeln!(o, "- Security ({}): {}", f.level.as_str(), f.message);
        }
    }

    let _ = writeln!(o, "\n### Resources\n");
    if let Some(c) = &m.cpu {
        let _ = writeln!(
            o,
            "- CPU {:.1}% (user {:.1}%, system {:.1}%, iowait {:.1}%, steal {:.1}%)",
            c.total_pct, c.user_pct, c.system_pct, c.iowait_pct, c.steal_pct
        );
        if !c.cores.is_empty() {
            let cores: Vec<String> = c.cores.iter().map(|v| format!("{v:.0}")).collect();
            let _ = writeln!(o, "- Per core %: {}", cores.join(" "));
        }
    }
    if let Some(l) = &m.load {
        let _ = writeln!(
            o,
            "- Load: {:.2} {:.2} {:.2} ({} running / {} tasks)",
            l.one, l.five, l.fifteen, l.running, l.total
        );
    }
    if let Some(mem) = &m.mem {
        let _ = writeln!(
            o,
            "- Memory: {} / {} ({:.1}%), cache {}; swap {} / {} ({:.1}%)",
            human_bytes(mem.used as f64),
            human_bytes(mem.total as f64),
            mem.used_pct,
            human_bytes(mem.cache as f64),
            human_bytes(mem.swap_used as f64),
            human_bytes(mem.swap_total as f64),
            mem.swap_pct
        );
    }

    if !m.disks.is_empty() {
        let _ = writeln!(o, "\n| Mount | Filesystem | Used | Size | Use% |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
        for d in &m.disks {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {:.1}% |",
                cell(&d.mount),
                cell(&d.filesystem),
                human_bytes(d.used as f64),
                human_bytes(d.total as f64),
                d.used_pct
            );
        }
    }
    if !m.disk_io.is_empty() {
        let _ = writeln!(o, "\n| Device | Read/s | Write/s | IOPS r/w | Util |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
        for d in &m.disk_io {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {:.0}/{:.0} | {:.0}% |",
                d.device,
                human_bytes(d.read_bps),
                human_bytes(d.write_bps),
                d.read_iops,
                d.write_iops,
                d.util_pct
            );
        }
    }
    if !m.net.is_empty() {
        let _ = writeln!(o, "\n| Interface | RX/s | TX/s | Errors | Drops |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
        for n in &m.net {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} |",
                n.iface,
                human_bytes(n.rx_bps),
                human_bytes(n.tx_bps),
                n.errors,
                n.drops
            );
        }
    }
    if !m.procs.is_empty() {
        let _ = writeln!(o, "\n### Top processes ({} total)\n", m.proc_count);
        let _ = writeln!(o, "| PID | User | Command | CPU% | Mem% | RSS |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- |");
        for p in &m.procs {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {:.1} | {} |",
                p.pid,
                cell(&p.user),
                cell(&p.name),
                p.cpu_pct
                    .map(|c| format!("{c:.1}"))
                    .unwrap_or_else(|| "…".into()),
                p.mem_pct,
                human_bytes(p.rss as f64)
            );
        }
    }

    let _ = writeln!(o, "\n### Services and containers\n");
    match &m.failed_units {
        Probe::Ok(u) if u.is_empty() => {
            let _ = writeln!(o, "- Failed units: none");
        }
        Probe::Ok(u) => {
            let _ = writeln!(o, "- Failed units:");
            for x in u {
                let _ = writeln!(
                    o,
                    "  - `{}` ({}/{}) {}",
                    x.unit, x.active, x.sub, x.description
                );
            }
        }
        p => {
            let _ = writeln!(o, "- Failed units: {}", probe_note(p));
        }
    }
    match &m.containers {
        Probe::Ok(c) if c.is_empty() => {
            let _ = writeln!(o, "- Containers: none");
        }
        Probe::Ok(c) => {
            let _ = writeln!(o, "\n| Container | Image | Status | CPU | Memory |");
            let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
            for x in c {
                let _ = writeln!(
                    o,
                    "| {} | {} | {} | {} | {} |",
                    cell(&x.name),
                    cell(&x.image),
                    cell(&x.status),
                    pct(x.cpu_pct),
                    cell(x.mem_usage.as_deref().unwrap_or("n/a"))
                );
            }
        }
        p => {
            let _ = writeln!(o, "- Containers: {}", probe_note(p));
        }
    }

    let _ = writeln!(o, "\n### Security\n");
    match &m.ports {
        Probe::Ok(p) => {
            let list: Vec<String> = p.iter().map(ToString::to_string).collect();
            let _ = writeln!(o, "- Listening TCP: {}", list.join(", "));
        }
        p => {
            let _ = writeln!(o, "- Listening TCP: {}", probe_note(p));
        }
    }
    match &m.failed_logins {
        Probe::Ok(l) => {
            let top: Vec<String> = l
                .top_sources
                .iter()
                .map(|(ip, n)| format!("{ip} ({n})"))
                .collect();
            let _ = writeln!(
                o,
                "- Failed SSH logins (24h, {}): {}{}",
                l.source,
                l.total,
                if top.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", top.join(", "))
                }
            );
        }
        p => {
            let _ = writeln!(o, "- Failed SSH logins: {}", probe_note(p));
        }
    }
    match &m.updates {
        Probe::Ok(u) => {
            let sec = u
                .security
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into());
            let _ = writeln!(
                o,
                "- Pending updates ({}): {} total, {} security",
                u.manager, u.total, sec
            );
        }
        p => {
            let _ = writeln!(o, "- Pending updates: {}", probe_note(p));
        }
    }
}

/// Posts a snapshot to every webhook with `snapshot = true`.
pub async fn post(snapshot: &Snapshot, hooks: &[Webhook]) -> Vec<String> {
    let client = crate::alert::http_client();
    let mut errors = Vec::new();
    for hook in hooks.iter().filter(|h| h.snapshot) {
        let body = match hook.kind {
            WebhookKind::Generic => serde_json::json!({
                "source": "skry",
                "kind": "snapshot",
                "snapshot": snapshot,
            }),
            kind => crate::alert::chat_payload(kind, &snapshot.summary()),
        };
        if let Err(e) = crate::alert::post(&client, hook, &body).await {
            errors.push(e);
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::RawSample;
    use crate::config::Target;
    use crate::engine::{ConnState, HostError};
    use crate::model::{Health, HostModel, Thresholds};
    use crate::ssh::FailureKind;
    use chrono::TimeZone;

    fn fixture_host() -> HostState {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/debian-systemd/full-root.txt"
        ));
        let raw = RawSample::parse(text, "0123456789abcdef");
        let ts = Utc.with_ymd_and_hms(2026, 9, 29, 1, 5, 40).unwrap();
        let mut model = HostModel::new();
        let mut m = model.update(&raw, ts);
        // Keep the report stable: rates depend on two samples.
        m.procs.truncate(3);
        let target = Target {
            spec: "backup01".into(),
            groups: vec!["production".into()],
            allowed_ports: None,
        };
        let mut h = HostState::new(&target, Some("root@10.0.0.7:22".into()));
        h.health = Health::evaluate(&m, &Thresholds::default());
        h.metrics = Some(m);
        h.conn = ConnState::Connected;
        h.refresh_status();
        h
    }

    fn down_host() -> HostState {
        let target = Target {
            spec: "db02".into(),
            groups: vec![],
            allowed_ports: None,
        };
        let mut h = HostState::new(&target, Some("admin@10.0.0.9:22".into()));
        h.conn = ConnState::Failed;
        h.error = Some(HostError {
            kind: FailureKind::Timeout,
            message: "timed out while connecting".into(),
            since: Utc.with_ymd_and_hms(2026, 9, 29, 1, 0, 0).unwrap(),
        });
        h.refresh_status();
        h
    }

    fn snapshot() -> Snapshot {
        let mut s = Snapshot::new(
            vec![fixture_host(), down_host()],
            vec![],
            Utc.with_ymd_and_hms(2026, 9, 29, 1, 6, 0).unwrap(),
        );
        s.skry_version = "test".into();
        s
    }

    #[test]
    fn markdown_report() {
        insta::assert_snapshot!(snapshot().to_markdown());
    }

    #[test]
    fn chat_summary() {
        insta::assert_snapshot!(snapshot().summary());
    }

    #[test]
    fn json_roundtrip_and_files() {
        let s = snapshot();
        let back: Snapshot = serde_json::from_str(&s.to_json()).unwrap();
        assert_eq!(back.hosts.len(), 2);
        assert_eq!(back.hosts[0].metrics, s.hosts[0].metrics);
        let dir = tempfile::tempdir().unwrap();
        let (md, json) = s.write(dir.path()).unwrap();
        assert!(md.ends_with("skry-snapshot-20260929-010600.md"));
        assert!(json.exists());
    }

    #[test]
    fn filter_hosts() {
        let fleet = FleetState {
            hosts: vec![fixture_host(), down_host()],
            ..Default::default()
        };
        let s = Snapshot::from_fleet(&fleet, Some(&["db02".to_string()]));
        assert_eq!(s.hosts.len(), 1);
        assert_eq!(s.hosts[0].name, "db02");
    }
}
