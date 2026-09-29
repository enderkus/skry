//! Target specifications (`host`, `user@host:port`, `[v6]:port`) and their
//! resolution against the SSH client configuration.

use std::path::PathBuf;

use super::SshError;
use super::sshconfig::{SshConfig, expand_tilde};

const MAX_JUMPS: usize = 8;

/// A parsed `[user@]host[:port]` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
}

impl TargetSpec {
    pub fn parse(s: &str) -> Result<TargetSpec, SshError> {
        let s = s.trim();
        let s = s.strip_prefix("ssh://").unwrap_or(s);
        let (user, rest) = match s.rsplit_once('@') {
            Some((u, r)) if !u.is_empty() => (Some(u.to_string()), r),
            Some((_, r)) => (None, r),
            None => (None, s),
        };
        let (host, port) = if let Some(inner) = rest.strip_prefix('[') {
            let (h, after) = inner
                .split_once(']')
                .ok_or_else(|| SshError::Config(format!("unterminated '[' in {s:?}")))?;
            let port = match after.strip_prefix(':') {
                Some(p) => Some(parse_port(p, s)?),
                None if after.is_empty() => None,
                None => return Err(SshError::Config(format!("invalid target {s:?}"))),
            };
            (h.to_string(), port)
        } else if rest.matches(':').count() == 1 {
            let (h, p) = rest.split_once(':').unwrap_or((rest, ""));
            (h.to_string(), Some(parse_port(p, s)?))
        } else {
            // No port, or a bare IPv6 address.
            (rest.to_string(), None)
        };
        if host.is_empty() || host.starts_with('-') || host.chars().any(char::is_whitespace) {
            return Err(SshError::Config(format!("invalid target {s:?}")));
        }
        Ok(TargetSpec { user, host, port })
    }
}

fn parse_port(p: &str, whole: &str) -> Result<u16, SshError> {
    p.parse::<u16>()
        .ok()
        .filter(|&p| p != 0)
        .ok_or_else(|| SshError::Config(format!("invalid port in {whole:?}")))
}

/// Everything needed to open a connection to one host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHost {
    /// The name as given by the user, used for display.
    pub name: String,
    /// Address to connect to.
    pub hostname: String,
    pub port: u16,
    pub user: String,
    pub identity_files: Vec<PathBuf>,
    pub identities_only: bool,
    /// Jump hosts in connection order.
    pub jumps: Vec<ResolvedHost>,
    pub known_hosts_files: Vec<PathBuf>,
    /// Name to look up in `known_hosts` (`HostKeyAlias` or the host name).
    pub known_hosts_name: String,
    pub connect_timeout: Option<u64>,
}

impl ResolvedHost {
    /// `user@host:port`, for messages.
    pub fn display_addr(&self) -> String {
        let host = if self.hostname.contains(':') {
            format!("[{}]", self.hostname)
        } else {
            self.hostname.clone()
        };
        format!("{}@{}:{}", self.user, host, self.port)
    }
}

pub fn local_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "root".into())
}

fn expand_tokens(s: &str, host: &str, hostname: &str, port: u16, user: &str, home: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('h') => out.push_str(hostname),
            Some('n') => out.push_str(host),
            Some('p') => out.push_str(&port.to_string()),
            Some('r') => out.push_str(user),
            Some('u') => out.push_str(&local_user()),
            Some('d') => out.push_str(home),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

/// Resolves a target against the SSH config, including its ProxyJump chain.
pub fn resolve(spec: &str, cfg: &SshConfig) -> Result<ResolvedHost, SshError> {
    resolve_inner(spec, cfg, 0)
}

fn resolve_inner(spec: &str, cfg: &SshConfig, depth: usize) -> Result<ResolvedHost, SshError> {
    if depth > MAX_JUMPS {
        return Err(SshError::Config(format!(
            "ProxyJump chain for {spec} is too long or circular"
        )));
    }
    let t = TargetSpec::parse(spec)?;
    let opts = cfg.query(&t.host);
    let home = cfg.home().to_path_buf();
    let home_s = home.to_string_lossy().to_string();
    let port = t.port.or(opts.port).unwrap_or(22);
    let user = t
        .user
        .clone()
        .or(opts.user.clone())
        .unwrap_or_else(local_user);
    let hostname = opts
        .hostname
        .as_deref()
        .map(|h| expand_tokens(h, &t.host, &t.host, port, &user, &home_s))
        .unwrap_or_else(|| t.host.clone());

    let identity_files: Vec<PathBuf> = if opts.identity_files.is_empty() {
        ["id_ed25519", "id_ecdsa", "id_rsa"]
            .iter()
            .map(|f| home.join(".ssh").join(f))
            .filter(|p| p.exists())
            .collect()
    } else {
        opts.identity_files
            .iter()
            .map(|f| {
                expand_tilde(
                    &expand_tokens(f, &t.host, &hostname, port, &user, &home_s),
                    &home,
                )
            })
            .collect()
    };

    let known_hosts_files = opts
        .user_known_hosts_files
        .clone()
        .filter(|f| !f.iter().any(|x| x.eq_ignore_ascii_case("none")))
        .map(|files| {
            files
                .iter()
                .map(|f| {
                    expand_tilde(
                        &expand_tokens(f, &t.host, &hostname, port, &user, &home_s),
                        &home,
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                home.join(".ssh").join("known_hosts"),
                home.join(".ssh").join("known_hosts2"),
            ]
        });

    let mut jumps = Vec::new();
    if let Some(pj) = opts.proxy_jump.as_deref()
        && !pj.eq_ignore_ascii_case("none")
    {
        for hop in pj.split(',').map(str::trim).filter(|h| !h.is_empty()) {
            let mut j = resolve_inner(hop, cfg, depth + 1)?;
            // A jump host's own jumps come first.
            jumps.append(&mut j.jumps);
            jumps.push(j);
        }
        if jumps.len() > MAX_JUMPS {
            return Err(SshError::Config(format!(
                "ProxyJump chain for {spec} is too long"
            )));
        }
    }

    Ok(ResolvedHost {
        name: spec.to_string(),
        known_hosts_name: opts
            .host_key_alias
            .clone()
            .unwrap_or_else(|| hostname.clone()),
        hostname,
        port,
        user,
        identity_files,
        identities_only: opts.identities_only.unwrap_or(false),
        jumps,
        known_hosts_files,
        connect_timeout: opts.connect_timeout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_specs() {
        let t = TargetSpec::parse("deploy@db1:2222").unwrap();
        assert_eq!(t.user.as_deref(), Some("deploy"));
        assert_eq!(t.host, "db1");
        assert_eq!(t.port, Some(2222));
        let t = TargetSpec::parse("web1").unwrap();
        assert_eq!((t.user, t.port), (None, None));
        let t = TargetSpec::parse("[2001:db8::1]:22").unwrap();
        assert_eq!(t.host, "2001:db8::1");
        assert_eq!(t.port, Some(22));
        let t = TargetSpec::parse("root@2001:db8::1").unwrap();
        assert_eq!(t.host, "2001:db8::1");
        assert_eq!(t.port, None);
        let t = TargetSpec::parse("ssh://me@h:2200").unwrap();
        assert_eq!(t.user.as_deref(), Some("me"));
        assert_eq!(t.port, Some(2200));
    }

    #[test]
    fn reject_bad_specs() {
        for bad in [
            "",
            "host:0",
            "host:99999",
            "-oProxyCommand=x",
            "a b",
            "[::1",
        ] {
            assert!(TargetSpec::parse(bad).is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn resolve_with_config_and_overrides() {
        let cfg = SshConfig::parse(
            "Host web1\n  HostName 10.1.1.1\n  User deploy\n  Port 2200\n  IdentityFile ~/.ssh/%n_key\n  HostKeyAlias web1-alias\n",
            Path::new("/home/u"),
        );
        let r = resolve("web1", &cfg).unwrap();
        assert_eq!(r.hostname, "10.1.1.1");
        assert_eq!(r.user, "deploy");
        assert_eq!(r.port, 2200);
        assert_eq!(
            r.identity_files,
            vec![PathBuf::from("/home/u/.ssh/web1_key")]
        );
        assert_eq!(r.known_hosts_name, "web1-alias");
        assert_eq!(
            r.known_hosts_files[0],
            PathBuf::from("/home/u/.ssh/known_hosts")
        );
        let r = resolve("admin@web1:22", &cfg).unwrap();
        assert_eq!(r.user, "admin");
        assert_eq!(r.port, 22);
        assert_eq!(r.display_addr(), "admin@10.1.1.1:22");
    }

    #[test]
    fn proxy_jump_chain() {
        let cfg = SshConfig::parse(
            "Host inner\n  ProxyJump mid\nHost mid\n  ProxyJump ops@edge:2222\nHost edge\n  HostName edge.example.com\n",
            Path::new("/home/u"),
        );
        let r = resolve("inner", &cfg).unwrap();
        let chain: Vec<String> = r.jumps.iter().map(|j| j.display_addr()).collect();
        assert_eq!(chain.len(), 2);
        assert!(chain[0].starts_with("ops@edge.example.com:2222"));
        assert_eq!(r.jumps[1].hostname, "mid");
        assert!(r.jumps.iter().all(|j| j.jumps.is_empty()));
    }

    #[test]
    fn circular_proxy_jump_fails() {
        let cfg = SshConfig::parse(
            "Host a\n  ProxyJump b\nHost b\n  ProxyJump a\n",
            Path::new("/home/u"),
        );
        assert!(resolve("a", &cfg).is_err());
    }

    #[test]
    fn proxy_jump_none() {
        let cfg = SshConfig::parse("Host *\n  ProxyJump none\n", Path::new("/home/u"));
        assert!(resolve("h", &cfg).unwrap().jumps.is_empty());
    }
}
