//! Security pulse: failed SSH logins, pending security updates, unexpected
//! listening ports, and TLS certificate expiry.
//!
//! Host findings come from the regular collection; TLS endpoints are
//! checked from the local machine with a real TLS handshake.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use serde::{Deserialize, Serialize};

use crate::collect::{FailedLogins, ListenPort, Probe, Updates};
use crate::config::SecurityConfig;
use crate::model::{HostMetrics, Level};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// `failed_logins`, `security_updates`, `unexpected_ports`.
    pub name: String,
    pub level: Level,
    pub message: String,
}

fn is_loopback(addr: &str) -> bool {
    addr.starts_with("127.")
        || addr == "::1"
        || addr == "localhost"
        || addr.starts_with("::ffff:127.")
}

/// Ports listening on non-loopback addresses that are not allowed.
pub fn unexpected_ports(ports: &[ListenPort], allowed: &BTreeSet<u16>) -> Vec<u16> {
    let set: BTreeSet<u16> = ports
        .iter()
        .filter(|p| !is_loopback(&p.addr) && !allowed.contains(&p.port))
        .map(|p| p.port)
        .collect();
    set.into_iter().collect()
}

pub fn host_findings(
    m: &HostMetrics,
    allowed: Option<&BTreeSet<u16>>,
    cfg: &SecurityConfig,
) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Probe::Ok(l) = &m.failed_logins
        && cfg.failed_login_threshold > 0
        && l.total >= cfg.failed_login_threshold
    {
        let level = if l.total >= cfg.failed_login_threshold.saturating_mul(10) {
            Level::Critical
        } else {
            Level::Warning
        };
        let top = l
            .top_sources
            .iter()
            .take(3)
            .map(|(ip, n)| format!("{ip} ×{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push(Finding {
            name: "failed_logins".into(),
            level,
            message: format!("{} failed SSH logins in 24h (top: {top})", l.total),
        });
    }
    if let Probe::Ok(u) = &m.updates
        && let Some(sec) = u.security
        && sec > 0
    {
        let sample = u
            .security_packages
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        out.push(Finding {
            name: "security_updates".into(),
            level: Level::Warning,
            message: format!("{sec} pending security updates ({sample})"),
        });
    }
    if let (Probe::Ok(ports), Some(allowed)) = (&m.ports, allowed) {
        let bad = unexpected_ports(ports, allowed);
        if !bad.is_empty() {
            out.push(Finding {
                name: "unexpected_ports".into(),
                level: Level::Warning,
                message: format!(
                    "listening on ports outside the allowlist: {}",
                    bad.iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }
    }
    out
}

/// Security view of one host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostSecurity {
    pub host: String,
    pub reachable: bool,
    pub error: Option<String>,
    pub failed_logins: Probe<FailedLogins>,
    pub updates: Probe<Updates>,
    pub ports: Probe<Vec<ListenPort>>,
    pub allowed_ports: Option<Vec<u16>>,
    pub unexpected_ports: Vec<u16>,
    pub findings: Vec<Finding>,
}

impl HostSecurity {
    pub fn from_metrics(
        host: &str,
        m: &HostMetrics,
        allowed: Option<&BTreeSet<u16>>,
        cfg: &SecurityConfig,
    ) -> Self {
        let unexpected = match (&m.ports, allowed) {
            (Probe::Ok(p), Some(a)) => unexpected_ports(p, a),
            _ => Vec::new(),
        };
        HostSecurity {
            host: host.to_string(),
            reachable: true,
            error: None,
            failed_logins: m.failed_logins.clone(),
            updates: m.updates.clone(),
            ports: m.ports.clone(),
            allowed_ports: allowed.map(|a| a.iter().copied().collect()),
            unexpected_ports: unexpected,
            findings: host_findings(m, allowed, cfg),
        }
    }

    pub fn unreachable(host: &str, error: String) -> Self {
        HostSecurity {
            host: host.to_string(),
            reachable: false,
            error: Some(error),
            failed_logins: Probe::Pending,
            updates: Probe::Pending,
            ports: Probe::Pending,
            allowed_ports: None,
            unexpected_ports: Vec::new(),
            findings: Vec::new(),
        }
    }

    pub fn level(&self) -> Level {
        self.findings
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TlsResult {
    pub endpoint: String,
    pub not_after: Option<DateTime<Utc>>,
    pub days_left: Option<i64>,
    pub subject: Option<String>,
    pub issuer: Option<String>,
    /// Whether the chain validates against the Mozilla root store for the
    /// endpoint's host name.
    pub valid: bool,
    pub problem: Option<String>,
    pub level: Level,
}

impl TlsResult {
    pub fn describe(&self) -> String {
        match (&self.problem, self.days_left) {
            (Some(p), _) if self.not_after.is_none() => p.clone(),
            (Some(p), Some(d)) => format!("{p}; expires in {d} days"),
            (None, Some(d)) => format!("expires in {d} days"),
            _ => "unknown".into(),
        }
    }
}

/// Records the certificate and the verification verdict, but lets the
/// handshake finish so expired or mis-issued certificates can be reported.
/// Handshake signatures are still verified.
#[derive(Debug)]
struct CaptureVerifier {
    inner: Arc<rustls::client::WebPkiServerVerifier>,
    cert: Mutex<Option<Vec<u8>>>,
    verdict: Mutex<Option<Result<(), String>>>,
}

impl ServerCertVerifier for CaptureVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.cert.lock().expect("lock") = Some(end_entity.to_vec());
        let verdict = self
            .inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
            .map(|_| ())
            .map_err(|e| e.to_string());
        *self.verdict.lock().expect("lock") = Some(verdict);
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Splits `host[:port]` (default port 443).
pub fn parse_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let e = endpoint.trim();
    let e = e.strip_prefix("https://").unwrap_or(e);
    let e = e.split('/').next().unwrap_or(e);
    if let Some(rest) = e.strip_prefix('[') {
        let (h, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None => 443,
        };
        return Some((h.to_string(), port));
    }
    match e.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => Some((h.to_string(), p.parse().ok()?)),
        _ if !e.is_empty() => Some((e.to_string(), 443)),
        _ => None,
    }
}

fn short_name(name: &x509_parser::x509::X509Name) -> String {
    name.iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok().map(str::to_string))
        .unwrap_or_else(|| name.to_string())
}

pub fn level_for(days_left: i64, valid: bool, cfg: &SecurityConfig) -> Level {
    if !valid || days_left <= cfg.tls_critical_days {
        Level::Critical
    } else if days_left <= cfg.tls_warning_days {
        Level::Warning
    } else {
        Level::Ok
    }
}

/// Performs a TLS handshake with `endpoint` and reports the leaf
/// certificate's expiry and validity.
pub async fn check_tls(endpoint: &str, timeout: Duration, cfg: &SecurityConfig) -> TlsResult {
    let mut result = TlsResult {
        endpoint: endpoint.to_string(),
        not_after: None,
        days_left: None,
        subject: None,
        issuer: None,
        valid: false,
        problem: None,
        level: Level::Critical,
    };
    let Some((host, port)) = parse_endpoint(endpoint) else {
        result.problem = Some("invalid endpoint".into());
        return result;
    };
    match tokio::time::timeout(timeout, handshake(&host, port)).await {
        Err(_) => result.problem = Some("timed out".into()),
        Ok(Err(e)) => result.problem = Some(e),
        Ok(Ok((der, verdict))) => {
            match x509_parser::parse_x509_certificate(&der) {
                Ok((_, cert)) => {
                    let ts = cert.validity().not_after.timestamp();
                    let not_after = DateTime::from_timestamp(ts, 0);
                    result.not_after = not_after;
                    result.days_left = not_after.map(|t| (t - Utc::now()).num_days());
                    result.subject = Some(short_name(cert.subject()));
                    result.issuer = Some(short_name(cert.issuer()));
                }
                Err(e) => result.problem = Some(format!("unparsable certificate: {e}")),
            }
            match verdict {
                Some(Ok(())) => result.valid = true,
                Some(Err(e)) => result.problem = Some(e),
                None => result.problem = Some("certificate not verified".into()),
            }
            if let Some(d) = result.days_left {
                result.level = level_for(d, result.valid, cfg);
            }
        }
    }
    result
}

type Verdict = Option<Result<(), String>>;

async fn handshake(host: &str, port: u16) -> Result<(Vec<u8>, Verdict), String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = Arc::new(rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    });
    let inner =
        rustls::client::WebPkiServerVerifier::builder_with_provider(roots, provider.clone())
            .build()
            .map_err(|e| e.to_string())?;
    let verifier = Arc::new(CaptureVerifier {
        inner,
        cert: Mutex::new(None),
        verdict: Mutex::new(None),
    });
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(verifier.clone())
        .with_no_client_auth();
    let name = ServerName::try_from(host.to_string()).map_err(|e| e.to_string())?;
    let tcp = tokio::net::TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("connect: {e}"))?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let mut tls = connector
        .connect(name, tcp)
        .await
        .map_err(|e| format!("handshake: {e}"))?;
    let _ = tokio::io::AsyncWriteExt::shutdown(&mut tls).await;
    let der = verifier
        .cert
        .lock()
        .expect("lock")
        .take()
        .ok_or("no certificate presented")?;
    let verdict = verifier.verdict.lock().expect("lock").take();
    Ok((der, verdict))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lp(port: u16, addr: &str) -> ListenPort {
        ListenPort {
            port,
            addr: addr.into(),
        }
    }

    #[test]
    fn unexpected_ports_ignore_loopback() {
        let ports = vec![
            lp(22, "0.0.0.0"),
            lp(22, "::"),
            lp(5432, "127.0.0.1"),
            lp(6379, "0.0.0.0"),
            lp(8080, "::1"),
            lp(9000, "10.0.0.5"),
        ];
        let allowed: BTreeSet<u16> = [22, 80].into();
        assert_eq!(unexpected_ports(&ports, &allowed), vec![6379, 9000]);
    }

    #[test]
    fn findings() {
        let m = HostMetrics {
            failed_logins: Probe::Ok(FailedLogins {
                source: "journal".into(),
                total: 120,
                top_sources: vec![("203.0.113.9".into(), 110), ("198.51.100.2".into(), 10)],
            }),
            updates: Probe::Ok(Updates {
                manager: "apt".into(),
                total: 4,
                security: Some(2),
                security_packages: vec!["openssl".into(), "libc6".into()],
            }),
            ports: Probe::Ok(vec![lp(22, "0.0.0.0"), lp(3306, "0.0.0.0")]),
            ..Default::default()
        };
        let cfg = SecurityConfig::default();
        let allowed: BTreeSet<u16> = [22].into();
        let f = host_findings(&m, Some(&allowed), &cfg);
        assert_eq!(f.len(), 3);
        assert_eq!(f[0].level, Level::Warning);
        assert!(f[0].message.contains("203.0.113.9 ×110"));
        assert!(f[1].message.contains("openssl, libc6"));
        assert!(f[2].message.ends_with("3306"));
        // No allowlist: no port finding.
        assert_eq!(host_findings(&m, None, &cfg).len(), 2);
        let hs = HostSecurity::from_metrics("h", &m, Some(&allowed), &cfg);
        assert_eq!(hs.unexpected_ports, vec![3306]);
        assert_eq!(hs.level(), Level::Warning);
    }

    #[test]
    fn many_failed_logins_are_critical() {
        let m = HostMetrics {
            failed_logins: Probe::Ok(FailedLogins {
                source: "journal".into(),
                total: 5000,
                top_sources: vec![],
            }),
            ..Default::default()
        };
        let f = host_findings(&m, None, &SecurityConfig::default());
        assert_eq!(f[0].level, Level::Critical);
    }

    #[test]
    fn endpoints() {
        assert_eq!(
            parse_endpoint("example.com"),
            Some(("example.com".into(), 443))
        );
        assert_eq!(
            parse_endpoint("example.com:8443"),
            Some(("example.com".into(), 8443))
        );
        assert_eq!(
            parse_endpoint("https://example.com/x"),
            Some(("example.com".into(), 443))
        );
        assert_eq!(parse_endpoint("[::1]:8443"), Some(("::1".into(), 8443)));
        assert_eq!(parse_endpoint(""), None);
    }

    #[test]
    fn tls_levels() {
        let cfg = SecurityConfig::default();
        assert_eq!(level_for(90, true, &cfg), Level::Ok);
        assert_eq!(level_for(20, true, &cfg), Level::Warning);
        assert_eq!(level_for(3, true, &cfg), Level::Critical);
        assert_eq!(level_for(90, false, &cfg), Level::Critical);
    }

    #[tokio::test]
    async fn tls_connection_refused_is_reported() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let r = check_tls(
            &format!("127.0.0.1:{port}"),
            Duration::from_secs(3),
            &SecurityConfig::default(),
        )
        .await;
        assert!(!r.valid);
        assert_eq!(r.level, Level::Critical);
        assert!(r.problem.is_some());
    }

    /// Hits the network; run with `SKRY_NETWORK_TESTS=1`.
    #[tokio::test]
    async fn tls_real_endpoint() {
        if std::env::var("SKRY_NETWORK_TESTS").is_err() {
            return;
        }
        let cfg = SecurityConfig::default();
        let ok = check_tls("www.rust-lang.org:443", Duration::from_secs(10), &cfg).await;
        assert!(ok.valid, "{ok:?}");
        assert!(ok.days_left.unwrap() > 0);
        let expired = check_tls("expired.badssl.com:443", Duration::from_secs(10), &cfg).await;
        assert!(!expired.valid);
        assert!(expired.days_left.unwrap() < 0);
        assert_eq!(expired.level, Level::Critical);
    }
}
