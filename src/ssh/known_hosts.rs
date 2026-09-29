//! Host key verification against OpenSSH `known_hosts` files.
//!
//! Handles hashed entries (`|1|salt|hash`), wildcard and negated patterns,
//! `[host]:port` entries, and `@revoked` markers. `@cert-authority` lines
//! are ignored.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, KeyInit, Mac};
use russh::keys::{Algorithm, PublicKey};
use sha1::Sha1;

use super::sshconfig::wildcard_match;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyStatus {
    /// The key is recorded for this host.
    Known,
    /// Nothing is recorded for this host.
    Unknown,
    /// Other keys are recorded for this host: possible man-in-the-middle.
    Changed { path: PathBuf, line: usize },
    /// The key is explicitly revoked.
    Revoked,
}

/// The name used in `known_hosts` for a host and port.
pub fn host_port_name(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

fn hashed_match(entry: &str, name: &str) -> bool {
    let mut parts = entry.split('|').skip(2);
    let (Some(salt), Some(hash)) = (parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(salt), Ok(hash)) = (B64.decode(salt), B64.decode(hash)) else {
        return false;
    };
    let Ok(mac) = Hmac::<Sha1>::new_from_slice(&salt) else {
        return false;
    };
    mac.chain_update(name.as_bytes())
        .verify_slice(&hash)
        .is_ok()
}

/// Whether a comma-separated host pattern list matches `name`.
fn hosts_match(patterns: &str, name: &str) -> bool {
    let mut matched = false;
    for p in patterns.split(',') {
        if p.starts_with("|1|") {
            if hashed_match(p, name) {
                matched = true;
            }
        } else if let Some(neg) = p.strip_prefix('!') {
            if wildcard_match(neg, name) {
                return false;
            }
        } else if wildcard_match(p, name) {
            matched = true;
        }
    }
    matched
}

struct Entry {
    line: usize,
    revoked: bool,
    key: Option<PublicKey>,
}

fn entries_for(path: &Path, name: &str) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let mut first = fields.next().unwrap_or("");
        let mut revoked = false;
        if first.starts_with('@') {
            match first {
                "@revoked" => revoked = true,
                _ => continue, // @cert-authority and unknown markers
            }
            first = fields.next().unwrap_or("");
        }
        if !hosts_match(first, name) {
            continue;
        }
        let (Some(_ktype), Some(b64)) = (fields.next(), fields.next()) else {
            continue;
        };
        out.push(Entry {
            line: i + 1,
            revoked,
            key: russh::keys::parse_public_key_base64(b64).ok(),
        });
    }
    out
}

/// Checks `key` for `host`:`port` against the given files.
pub fn check(files: &[PathBuf], host: &str, port: u16, key: &PublicKey) -> HostKeyStatus {
    let name = host_port_name(host, port);
    let mut changed = None;
    let mut known = false;
    for path in files {
        for e in entries_for(path, &name) {
            let same = e
                .key
                .as_ref()
                .is_some_and(|k| k.key_data() == key.key_data());
            if e.revoked {
                if same {
                    return HostKeyStatus::Revoked;
                }
                continue;
            }
            if same {
                known = true;
            } else if changed.is_none() {
                changed = Some(HostKeyStatus::Changed {
                    path: path.clone(),
                    line: e.line,
                });
            }
        }
    }
    if known {
        HostKeyStatus::Known
    } else {
        changed.unwrap_or(HostKeyStatus::Unknown)
    }
}

/// Algorithms of the keys recorded for a host, so negotiation can prefer
/// them (as OpenSSH does) instead of tripping over a different key type.
pub fn known_algorithms(files: &[PathBuf], host: &str, port: u16) -> Vec<Algorithm> {
    let name = host_port_name(host, port);
    let mut out: Vec<Algorithm> = Vec::new();
    for path in files {
        for e in entries_for(path, &name) {
            if let Some(k) = e.key.filter(|_| !e.revoked) {
                let alg = k.algorithm();
                if !out.contains(&alg) {
                    out.push(alg);
                }
            }
        }
    }
    out
}

/// Appends a host key (as used by `--accept-new`). Safe to call from many
/// connections at once: writes are serialised and each line is written in
/// a single call.
pub fn learn(path: &Path, host: &str, port: u16, key: &PublicKey) -> std::io::Result<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let encoded = key
        .to_openssh()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    // Drop any comment carried by the key.
    let mut parts = encoded.split_whitespace();
    let (ktype, data) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let needs_newline = std::fs::read(path)
        .map(|b| !b.is_empty() && !b.ends_with(b"\n"))
        .unwrap_or(false);
    let mut line = String::new();
    if needs_newline {
        line.push('\n');
    }
    line.push_str(&format!("{} {ktype} {data}\n", host_port_name(host, port)));
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)?.write_all(line.as_bytes())
}

/// SHA256 fingerprint as printed by OpenSSH.
pub fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(russh::keys::HashAlg::Sha256).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ED1: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIG+jDkXzDAML+meMRLYztjniTgabJ/ejAVZ3JTyYqqEg";
    const ED2: &str = "AAAAC3NzaC1lZDI1NTE5AAAAILB2IPtajJEEEZLKvSKp+FvvFsf7TKWVYaIS7h1bh+FM";

    fn key(b64: &str) -> PublicKey {
        russh::keys::parse_public_key_base64(b64).unwrap()
    }

    fn file(content: &str) -> (tempfile::TempDir, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("known_hosts");
        std::fs::write(&p, content).unwrap();
        (dir, vec![p])
    }

    #[test]
    fn known_unknown_changed() {
        let (_d, files) = file(&format!(
            "# comment\nweb1,10.0.0.1 ssh-ed25519 {ED1}\n[web2]:2222 ssh-ed25519 {ED2} note\n"
        ));
        assert_eq!(check(&files, "web1", 22, &key(ED1)), HostKeyStatus::Known);
        assert_eq!(
            check(&files, "10.0.0.1", 22, &key(ED1)),
            HostKeyStatus::Known
        );
        assert_eq!(check(&files, "web2", 2222, &key(ED2)), HostKeyStatus::Known);
        assert_eq!(check(&files, "web2", 22, &key(ED2)), HostKeyStatus::Unknown);
        assert_eq!(
            check(&files, "web1", 22, &key(ED2)),
            HostKeyStatus::Changed {
                path: files[0].clone(),
                line: 2
            }
        );
    }

    #[test]
    fn hashed_entries() {
        // Same construction as `ssh-keygen -H`: HMAC-SHA1 keyed by the salt.
        let salt = [7u8; 20];
        let mut mac = Hmac::<Sha1>::new_from_slice(&salt).unwrap();
        mac.update(b"hashed.example");
        let hash = mac.finalize().into_bytes();
        let line = format!(
            "|1|{}|{} ssh-ed25519 {ED1}\n",
            B64.encode(salt),
            B64.encode(hash)
        );
        let (_d, files) = file(&line);
        assert_eq!(
            check(&files, "hashed.example", 22, &key(ED1)),
            HostKeyStatus::Known
        );
        assert_eq!(
            check(&files, "other.example", 22, &key(ED1)),
            HostKeyStatus::Unknown
        );
    }

    #[test]
    fn wildcards_and_negation() {
        let (_d, files) = file(&format!(
            "*.example.com,!evil.example.com ssh-ed25519 {ED1}\n"
        ));
        assert_eq!(
            check(&files, "a.example.com", 22, &key(ED1)),
            HostKeyStatus::Known
        );
        assert_eq!(
            check(&files, "evil.example.com", 22, &key(ED1)),
            HostKeyStatus::Unknown
        );
    }

    #[test]
    fn revoked_and_cert_authority() {
        let (_d, files) = file(&format!(
            "@cert-authority *.example.com ssh-ed25519 {ED2}\n@revoked * ssh-ed25519 {ED1}\nh ssh-ed25519 {ED1}\n"
        ));
        assert_eq!(check(&files, "h", 22, &key(ED1)), HostKeyStatus::Revoked);
        assert_eq!(
            check(&files, "x.example.com", 22, &key(ED2)),
            HostKeyStatus::Unknown
        );
    }

    #[test]
    fn learn_then_known() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub/known_hosts");
        learn(&p, "new", 2200, &key(ED1)).unwrap();
        let files = vec![p.clone()];
        assert_eq!(check(&files, "new", 2200, &key(ED1)), HostKeyStatus::Known);
        assert_eq!(
            known_algorithms(&files, "new", 2200),
            vec![Algorithm::Ed25519]
        );
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("[new]:2200 ssh-ed25519 AAAA"));
    }

    #[test]
    fn concurrent_learning_keeps_lines_intact() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("known_hosts");
        let handles: Vec<_> = (0..16)
            .map(|i| {
                let p = p.clone();
                std::thread::spawn(move || learn(&p, "h", 2200 + i, &key(ED1)).unwrap())
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let files = vec![p.clone()];
        for i in 0..16 {
            assert_eq!(
                check(&files, "h", 2200 + i, &key(ED1)),
                HostKeyStatus::Known
            );
        }
        assert_eq!(std::fs::read_to_string(&p).unwrap().lines().count(), 16);
    }

    #[test]
    fn missing_file_is_unknown() {
        let files = vec![PathBuf::from("/nonexistent/known_hosts")];
        assert_eq!(check(&files, "h", 22, &key(ED1)), HostKeyStatus::Unknown);
    }

    #[test]
    fn fingerprint_format() {
        assert!(fingerprint(&key(ED1)).starts_with("SHA256:"));
    }
}
