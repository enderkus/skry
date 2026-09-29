//! A small OpenSSH client configuration parser.
//!
//! Supported: `Host` patterns (with `*`, `?` and `!` negation), `Match all`,
//! `Include` (with globs, relative to `~/.ssh`), `HostName`, `User`, `Port`,
//! `IdentityFile`, `IdentitiesOnly`, `ProxyJump`, `UserKnownHostsFile`,
//! `ConnectTimeout` and `HostKeyAlias`. Other keywords are ignored. As in
//! OpenSSH, the first obtained value for each option wins, except
//! `IdentityFile`, which accumulates.

use std::path::{Path, PathBuf};

const MAX_INCLUDE_DEPTH: usize = 16;

#[derive(Debug, Clone, Default)]
struct Block {
    /// `None` for the implicit global block and for `Match all`.
    patterns: Option<Vec<String>>,
    /// Blocks we cannot evaluate (`Match exec …`) never apply.
    never: bool,
    options: Vec<(String, Vec<String>)>,
}

impl Block {
    fn matches(&self, host: &str) -> bool {
        if self.never {
            return false;
        }
        let Some(patterns) = &self.patterns else {
            return true;
        };
        let mut matched = false;
        for p in patterns {
            if let Some(neg) = p.strip_prefix('!') {
                if wildcard_match(neg, host) {
                    return false;
                }
            } else if wildcard_match(p, host) {
                matched = true;
            }
        }
        matched
    }
}

/// Options that apply to one host after evaluating the config.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostOptions {
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_files: Vec<String>,
    pub identities_only: Option<bool>,
    pub proxy_jump: Option<String>,
    pub user_known_hosts_files: Option<Vec<String>>,
    pub connect_timeout: Option<u64>,
    pub host_key_alias: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SshConfig {
    blocks: Vec<Block>,
    home: PathBuf,
}

/// Glob-style matching supporting `*` and `?`, case-insensitive like OpenSSH.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Splits a config line into keyword and arguments, honouring double quotes
/// and the `Keyword=value` form.
fn tokenize(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (keyword, rest) = match line.find(|c: char| c.is_whitespace() || c == '=') {
        Some(i) => (&line[..i], &line[i..]),
        None => (line, ""),
    };
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for c in rest.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            '#' if !in_quotes && !has_token => break,
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    args.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(cur);
    }
    Some((keyword.to_lowercase(), args))
}

pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

impl SshConfig {
    pub fn empty(home: PathBuf) -> Self {
        SshConfig {
            blocks: Vec::new(),
            home,
        }
    }

    /// Loads `~/.ssh/config` (or `path`). A missing file yields an empty config.
    pub fn load(path: Option<&Path>, home: &Path) -> std::io::Result<Self> {
        let path = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| home.join(".ssh").join("config"));
        let mut cfg = SshConfig::empty(home.to_path_buf());
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let mut current = Block::default();
                cfg.parse_into(&text, &mut current, 0);
                cfg.blocks.push(current);
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(cfg),
            Err(e) => Err(e),
        }
    }

    pub fn parse(text: &str, home: &Path) -> Self {
        let mut cfg = SshConfig::empty(home.to_path_buf());
        let mut current = Block::default();
        cfg.parse_into(text, &mut current, 0);
        cfg.blocks.push(current);
        cfg
    }

    fn parse_into(&mut self, text: &str, current: &mut Block, depth: usize) {
        for line in text.lines() {
            let Some((keyword, args)) = tokenize(line) else {
                continue;
            };
            match keyword.as_str() {
                "host" => {
                    let prev = std::mem::take(current);
                    self.blocks.push(prev);
                    current.patterns = Some(args);
                }
                "match" => {
                    let prev = std::mem::take(current);
                    self.blocks.push(prev);
                    let all = args.len() == 1 && args[0].eq_ignore_ascii_case("all");
                    current.never = !all;
                }
                "include" => {
                    if depth >= MAX_INCLUDE_DEPTH {
                        continue;
                    }
                    for arg in args {
                        let p = expand_tilde(&arg, &self.home);
                        let p = if p.is_absolute() {
                            p
                        } else {
                            self.home.join(".ssh").join(p)
                        };
                        let mut files: Vec<PathBuf> = glob::glob(&p.to_string_lossy())
                            .map(|g| g.filter_map(Result::ok).collect())
                            .unwrap_or_default();
                        files.sort();
                        for f in files {
                            if let Ok(text) = std::fs::read_to_string(&f) {
                                self.parse_into(&text, current, depth + 1);
                            }
                        }
                    }
                }
                _ => current.options.push((keyword, args)),
            }
        }
    }

    /// Evaluates the configuration for `host` (the name given by the user).
    pub fn query(&self, host: &str) -> HostOptions {
        let mut o = HostOptions::default();
        for block in self.blocks.iter().filter(|b| b.matches(host)) {
            for (k, args) in &block.options {
                let first = args.first().cloned();
                match k.as_str() {
                    "hostname" if o.hostname.is_none() => o.hostname = first,
                    "user" if o.user.is_none() => o.user = first,
                    "port" if o.port.is_none() => o.port = first.and_then(|p| p.parse().ok()),
                    "identityfile" => {
                        if let Some(f) = first
                            && !o.identity_files.contains(&f)
                        {
                            o.identity_files.push(f);
                        }
                    }
                    "identitiesonly" if o.identities_only.is_none() => {
                        o.identities_only = first.map(|v| v.eq_ignore_ascii_case("yes"))
                    }
                    "proxyjump" if o.proxy_jump.is_none() => o.proxy_jump = first,
                    "userknownhostsfile" if o.user_known_hosts_files.is_none() => {
                        o.user_known_hosts_files = Some(args.clone())
                    }
                    "connecttimeout" if o.connect_timeout.is_none() => {
                        o.connect_timeout = first.and_then(|v| v.parse().ok())
                    }
                    "hostkeyalias" if o.host_key_alias.is_none() => o.host_key_alias = first,
                    _ => {}
                }
            }
        }
        o
    }

    pub fn home(&self) -> &Path {
        &self.home
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        PathBuf::from("/home/u")
    }

    #[test]
    fn wildcard() {
        assert!(wildcard_match("*.example.com", "web.example.com"));
        assert!(wildcard_match("web?", "web1"));
        assert!(!wildcard_match("web?", "web10"));
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("WEB*", "web1"));
        assert!(!wildcard_match("db*", "web1"));
    }

    #[test]
    fn first_value_wins_and_identity_accumulates() {
        let cfg = SshConfig::parse(
            r#"
# comment
Host web1
    HostName 10.0.0.1
    User deploy
    IdentityFile ~/.ssh/web
Host web*
    User other
    Port 2222
    IdentityFile ~/.ssh/common
Host *
    User fallback
    IdentitiesOnly yes
"#,
            &home(),
        );
        let o = cfg.query("web1");
        assert_eq!(o.hostname.as_deref(), Some("10.0.0.1"));
        assert_eq!(o.user.as_deref(), Some("deploy"));
        assert_eq!(o.port, Some(2222));
        assert_eq!(o.identity_files, vec!["~/.ssh/web", "~/.ssh/common"]);
        assert_eq!(o.identities_only, Some(true));
        let o = cfg.query("db1");
        assert_eq!(o.user.as_deref(), Some("fallback"));
        assert_eq!(o.hostname, None);
    }

    #[test]
    fn negation_and_equals_syntax() {
        let cfg = SshConfig::parse(
            "Host *.prod !bastion.prod\n  ProxyJump=bastion.prod\nHost \"quoted host\"\n  User q\n",
            &home(),
        );
        assert_eq!(
            cfg.query("app.prod").proxy_jump.as_deref(),
            Some("bastion.prod")
        );
        assert_eq!(cfg.query("bastion.prod").proxy_jump, None);
        assert_eq!(cfg.query("quoted host").user.as_deref(), Some("q"));
    }

    #[test]
    fn global_options_before_host() {
        let cfg = SshConfig::parse("User everyone\nHost x\n  User xuser\n", &home());
        assert_eq!(cfg.query("x").user.as_deref(), Some("everyone"));
    }

    #[test]
    fn match_blocks_are_skipped_except_all() {
        let cfg = SshConfig::parse(
            "Match exec \"true\"\n  User nope\nMatch all\n  Port 2200\n",
            &home(),
        );
        let o = cfg.query("h");
        assert_eq!(o.user, None);
        assert_eq!(o.port, Some(2200));
    }

    #[test]
    fn include_with_glob() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().to_path_buf();
        std::fs::create_dir_all(home.join(".ssh/conf.d")).unwrap();
        std::fs::write(
            home.join(".ssh/conf.d/a.conf"),
            "Host a\n  HostName a.internal\n",
        )
        .unwrap();
        std::fs::write(
            home.join(".ssh/conf.d/b.conf"),
            "Host b\n  HostName b.internal\n",
        )
        .unwrap();
        std::fs::write(
            home.join(".ssh/config"),
            "Include conf.d/*.conf\nHost c\n  HostName c.internal\n",
        )
        .unwrap();
        let cfg = SshConfig::load(None, &home).unwrap();
        assert_eq!(cfg.query("a").hostname.as_deref(), Some("a.internal"));
        assert_eq!(cfg.query("b").hostname.as_deref(), Some("b.internal"));
        assert_eq!(cfg.query("c").hostname.as_deref(), Some("c.internal"));
    }

    #[test]
    fn recursive_include_terminates() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().to_path_buf();
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::write(home.join(".ssh/config"), "Include config\nUser x\n").unwrap();
        let cfg = SshConfig::load(None, &home).unwrap();
        assert_eq!(cfg.query("h").user.as_deref(), Some("x"));
    }

    #[test]
    fn missing_file_is_empty() {
        let cfg = SshConfig::load(Some(Path::new("/nonexistent/skry/config")), &home()).unwrap();
        assert_eq!(cfg.query("h"), HostOptions::default());
    }
}
