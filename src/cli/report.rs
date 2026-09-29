//! Plain-text tables for the query and report commands.

use skry::collect::Probe;
use skry::engine::{ConnState, HostState};
use skry::security::{HostSecurity, TlsResult};
use skry::util::{human_duration, pct};

/// Formats rows as aligned columns; the last column is never padded.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(c.chars().count());
        }
    }
    let line = |cells: Vec<&str>| {
        let mut s = String::new();
        for (i, c) in cells.iter().enumerate() {
            if i + 1 == cols {
                s.push_str(c);
            } else {
                s.push_str(&format!("{c:<w$}  ", w = widths[i]));
            }
        }
        s.trim_end().to_string()
    };
    let mut out = line(headers.to_vec());
    out.push('\n');
    for r in rows {
        out.push_str(&line(r.iter().map(String::as_str).collect()));
        out.push('\n');
    }
    out
}

pub fn error_of(h: &HostState) -> Option<String> {
    (h.conn == ConnState::Failed).then(|| {
        h.error
            .as_ref()
            .map(|e| format!("{}: {}", e.kind.label(), e.message))
            .unwrap_or_else(|| "unreachable".into())
    })
}

pub fn fleet(hosts: &[HostState]) -> String {
    let rows: Vec<Vec<String>> = hosts
        .iter()
        .map(|h| {
            let m = h.metrics.as_ref().filter(|_| h.conn != ConnState::Failed);
            let tail = error_of(h).unwrap_or_else(|| {
                let mut notes: Vec<String> =
                    h.health.breaches.iter().map(|b| b.describe()).collect();
                notes.extend(h.findings.iter().map(|f| f.message.clone()));
                if notes.is_empty() {
                    m.and_then(|m| m.os.clone()).unwrap_or_default()
                } else {
                    notes.join("; ")
                }
            });
            vec![
                h.name.clone(),
                h.status.label().to_string(),
                pct(m.and_then(|m| m.cpu_pct())),
                pct(m.and_then(|m| m.mem_pct())),
                pct(m.and_then(|m| m.disk_max_pct())),
                m.and_then(|m| m.load.map(|l| format!("{:.2}", l.one)))
                    .unwrap_or_else(|| "n/a".into()),
                m.and_then(|m| m.uptime_secs)
                    .map(human_duration)
                    .unwrap_or_else(|| "n/a".into()),
                tail,
            ]
        })
        .collect();
    table(
        &[
            "HOST", "STATUS", "CPU", "MEM", "DISK", "LOAD", "UPTIME", "DETAILS",
        ],
        &rows,
    )
}

pub fn security(hosts: &[HostSecurity], tls: &[TlsResult]) -> String {
    let rows: Vec<Vec<String>> = hosts
        .iter()
        .map(|h| {
            if !h.reachable {
                return vec![
                    h.host.clone(),
                    "unreachable".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    h.error.clone().unwrap_or_default(),
                ];
            }
            let logins = match &h.failed_logins {
                Probe::Ok(l) => {
                    let top = l
                        .top_sources
                        .first()
                        .map(|(ip, n)| format!(" (top {ip} ×{n})"))
                        .unwrap_or_default();
                    format!("{}{top}", l.total)
                }
                p => p.describe().unwrap_or_default(),
            };
            let updates = match &h.updates {
                Probe::Ok(u) => format!(
                    "{} security / {} total",
                    u.security
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "?".into()),
                    u.total
                ),
                p => p.describe().unwrap_or_default(),
            };
            let ports = match (&h.ports, &h.allowed_ports) {
                (Probe::Ok(_), Some(_)) if h.unexpected_ports.is_empty() => {
                    "as allowed".to_string()
                }
                (Probe::Ok(_), Some(_)) => format!(
                    "unexpected: {}",
                    h.unexpected_ports
                        .iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                (Probe::Ok(p), None) => {
                    let mut ports: Vec<u16> = p.iter().map(|x| x.port).collect();
                    ports.sort_unstable();
                    ports.dedup();
                    ports
                        .iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                }
                (p, _) => p.describe().unwrap_or_default(),
            };
            vec![
                h.host.clone(),
                h.level().as_str().to_string(),
                logins,
                updates,
                ports,
                h.findings
                    .iter()
                    .map(|f| f.message.clone())
                    .collect::<Vec<_>>()
                    .join("; "),
            ]
        })
        .collect();
    let mut out = table(
        &[
            "HOST",
            "PULSE",
            "FAILED LOGINS 24H",
            "UPDATES",
            "LISTENING",
            "FINDINGS",
        ],
        &rows,
    );
    if !tls.is_empty() {
        out.push('\n');
        let rows: Vec<Vec<String>> = tls
            .iter()
            .map(|t| {
                vec![
                    t.endpoint.clone(),
                    t.level.as_str().to_string(),
                    t.describe(),
                    t.issuer.clone().unwrap_or_default(),
                ]
            })
            .collect();
        out.push_str(&table(
            &["TLS ENDPOINT", "STATUS", "EXPIRY", "ISSUER"],
            &rows,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_columns() {
        let t = table(
            &["A", "BB", "C"],
            &[
                vec!["x".into(), "y".into(), "last column".into()],
                vec!["longer".into(), "".into(), "z".into()],
            ],
        );
        assert_eq!(t, "A       BB  C\nx       y   last column\nlonger      z\n");
    }
}
