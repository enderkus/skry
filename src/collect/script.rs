//! Builds the single POSIX `sh` command that is executed on the remote host
//! for every collection tick.
//!
//! Every section is introduced by a marker line of the form
//! `@@SKRY-<nonce>:<name>@@`. The nonce is random per invocation so that
//! nothing printed by the remote host (process names, log lines, container
//! names) can forge a section boundary. Commands that are missing or not
//! permitted simply print nothing and the parser reports "n/a".
//!
//! The script only reads: it uses `cat` on `/proc`, a handful of standard
//! inspection commands, and package-manager queries that work from cached
//! metadata. Nothing is written on the remote host.

use std::fmt::Write as _;

/// One unit of remote data collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    Meta,
    OsRelease,
    CpuInfo,
    Stat,
    MemInfo,
    LoadAvg,
    Uptime,
    NetDev,
    DiskStats,
    Df,
    IpAddr,
    ProcStat,
    Ps,
    PsArgs,
    Ports,
    Containers,
    FailedUnits,
    FailedLogins,
    Updates { rpm: bool },
    Unit(String),
}

impl Section {
    /// Marker name used in the output.
    pub fn name(&self) -> &'static str {
        match self {
            Section::Meta => "meta",
            Section::OsRelease => "os",
            Section::CpuInfo => "cpuinfo",
            Section::Stat => "stat",
            Section::MemInfo => "meminfo",
            Section::LoadAvg => "loadavg",
            Section::Uptime => "uptime",
            Section::NetDev => "netdev",
            Section::DiskStats => "diskstats",
            Section::Df => "df",
            Section::IpAddr => "ipaddr",
            Section::ProcStat => "procstat",
            Section::Ps => "ps",
            Section::PsArgs => "psargs",
            Section::Ports => "ports",
            Section::Containers => "containers",
            Section::FailedUnits => "units",
            Section::FailedLogins => "logins",
            Section::Updates { .. } => "updates",
            Section::Unit(_) => "unit",
        }
    }

    /// Sections collected on every tick.
    pub fn fast() -> Vec<Section> {
        vec![
            Section::Meta,
            Section::Stat,
            Section::MemInfo,
            Section::LoadAvg,
            Section::Uptime,
            Section::NetDev,
            Section::DiskStats,
            Section::Df,
            Section::ProcStat,
            Section::Ps,
        ]
    }

    /// Sections that change slowly or are expensive; collected every few ticks.
    pub fn slow(rpm_updates: bool) -> Vec<Section> {
        vec![
            Section::OsRelease,
            Section::CpuInfo,
            Section::IpAddr,
            Section::Ports,
            Section::Containers,
            Section::FailedUnits,
            Section::FailedLogins,
            Section::Updates { rpm: rpm_updates },
        ]
    }

    /// Everything needed for a full host picture.
    pub fn all(rpm_updates: bool) -> Vec<Section> {
        let mut v = Section::fast();
        v.extend(Section::slow(rpm_updates));
        v
    }

    fn body(&self) -> String {
        match self {
            Section::Meta => META.to_string(),
            Section::OsRelease => "cat /etc/os-release 2>/dev/null || cat /usr/lib/os-release 2>/dev/null".into(),
            Section::CpuInfo => {
                "grep -i -E '^(model name|hardware|cpu model)' /proc/cpuinfo 2>/dev/null | head -n 1".into()
            }
            Section::Stat => "grep -E '^(cpu|btime|procs_)' /proc/stat 2>/dev/null".into(),
            Section::MemInfo => "cat /proc/meminfo 2>/dev/null".into(),
            Section::LoadAvg => "cat /proc/loadavg 2>/dev/null".into(),
            Section::Uptime => "cat /proc/uptime 2>/dev/null".into(),
            Section::NetDev => "cat /proc/net/dev 2>/dev/null".into(),
            Section::DiskStats => "cat /proc/diskstats 2>/dev/null".into(),
            Section::Df => "$T df -P -k -l 2>/dev/null || $T df -P -k 2>/dev/null".into(),
            Section::IpAddr => "ip -o addr show 2>/dev/null".into(),
            Section::ProcStat => PROCSTAT.to_string(),
            Section::Ps => "ps -A -o pid= -o user= 2>/dev/null || ps -o pid,user 2>/dev/null".into(),
            Section::PsArgs => {
                "ps -A -o pid= -o user= -o args= 2>/dev/null || ps -o pid,user,args 2>/dev/null".into()
            }
            Section::Ports => PORTS.to_string(),
            Section::Containers => CONTAINERS.to_string(),
            Section::FailedUnits => UNITS.to_string(),
            Section::FailedLogins => LOGINS.to_string(),
            Section::Updates { rpm } => updates(*rpm),
            Section::Unit(name) => unit(name),
        }
    }
}

const META: &str = r##"printf 'hostname=%s\n' "$(hostname 2>/dev/null || cat /proc/sys/kernel/hostname 2>/dev/null)"
printf 'kernel=%s\n' "$(uname -r 2>/dev/null)"
printf 'arch=%s\n' "$(uname -m 2>/dev/null)"
printf 'time=%s\n' "$(date +%s 2>/dev/null)"
printf 'localtime=%s\n' "$(date +%Y-%m-%dT%H 2>/dev/null)"
printf 'pagesize=%s\n' "$(getconf PAGESIZE 2>/dev/null || getconf PAGE_SIZE 2>/dev/null)"
printf 'clk_tck=%s\n' "$(getconf CLK_TCK 2>/dev/null)"
printf 'uid=%s\n' "$(id -u 2>/dev/null)""##;

// `comm` may contain spaces and parentheses, so the fields after the last
// `)` are split separately. Output: pid state utime stime rss comm
const PROCSTAT: &str = r##"cat /proc/[0-9]*/stat 2>/dev/null | awk '{
  l = $0; p = index(l, "("); q = length(l)
  while (q > p && substr(l, q, 1) != ")") q--
  if (p == 0 || q <= p) next
  n = split(substr(l, q + 2), f, " ")
  if (n < 22) next
  print $1, f[1], f[12], f[13], f[22], substr(l, p + 1, q - p - 1)
}'"##;

const PORTS: &str = "ss -tlnH 2>/dev/null || ss -tln 2>/dev/null || netstat -tln 2>/dev/null || cat /proc/net/tcp /proc/net/tcp6 2>/dev/null";

const CONTAINERS: &str = r##"for r in docker podman; do
  command -v "$r" >/dev/null 2>&1 || continue
  echo "#runtime $r"
  if out=$($T "$r" ps -a --format '{{.Names}}|{{.Image}}|{{.Status}}|{{.State}}' 2>&1); then
    printf '%s\n' "$out" | sed '/^$/d; s/^/c|/'
    $T "$r" stats --no-stream --format '{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}' 2>/dev/null | sed '/^$/d; s/^/s|/'
  else
    printf '%s\n' "$out" | head -n 1 | sed 's/^/#error /'
  fi
done"##;

const UNITS: &str = r##"if [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then
  echo "#systemd"
  systemctl --failed --plain --no-legend --no-pager 2>/dev/null
  echo "#rc $?"
fi"##;

// Aggregates failed SSH authentication events by source address and hour so
// that only a small summary crosses the wire even under brute force.
const LOGIN_AWK: &str = r##"awk '
/not seeing messages from other users|insufficient permissions|No journal files were/ { print "#denied"; next }
/Failed password|Failed publickey|Failed keyboard-interactive|Invalid user/ {
  if ($1 ~ /^[0-9][0-9][0-9][0-9]-/) k = substr($1, 1, 13); else k = $1 " " $2 " " substr($3, 1, 2)
  ip = "-"
  for (i = 1; i < NF; i++) if ($i == "from") { ip = $(i + 1); break }
  c[ip " " k]++
}
END { for (x in c) print "k", c[x], x }' | head -n 5000"##;

fn logins_script() -> String {
    format!(
        r##"if command -v journalctl >/dev/null 2>&1 && [ -d /run/systemd/system ]; then
  echo "#source journal"
  $T journalctl --no-pager --since=-24h -o short-iso -t sshd -t sshd-session 2>&1 | {awk}
else
  f=""
  for c in /var/log/auth.log /var/log/secure /var/log/messages; do
    if [ -r "$c" ]; then f="$c"; break; fi
  done
  if [ -n "$f" ]; then
    echo "#source $f"
    files="$f"
    [ -r "$f.1" ] && files="$f.1 $f"
    cat $files 2>/dev/null | grep -E 'sshd' | {awk}
  elif [ -e /var/log/auth.log ] || [ -e /var/log/secure ]; then
    echo "#denied"
  fi
fi"##,
        awk = LOGIN_AWK
    )
}

static LOGINS: std::sync::LazyLock<String> = std::sync::LazyLock::new(logins_script);

fn updates(rpm: bool) -> String {
    let mut s = String::from(
        r##"if command -v apt-get >/dev/null 2>&1; then
  echo "#manager apt"
  $T apt-get -s -o Debug::NoLocking=1 dist-upgrade 2>/dev/null | grep '^Inst '
elif command -v apk >/dev/null 2>&1; then
  echo "#manager apk"
  $T apk version -l '<' 2>/dev/null | grep -v '^Installed'
"##,
    );
    if rpm {
        s.push_str(
            r##"elif command -v dnf >/dev/null 2>&1; then
  if [ "$(id -u)" = 0 ]; then
    echo "#manager dnf"
    $T dnf -C -q updateinfo list 2>/dev/null
  else
    echo "#denied"
  fi
"##,
        );
    } else {
        s.push_str(
            r##"elif command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1; then
  echo "#disabled"
"##,
        );
    }
    s.push_str("fi");
    s
}

/// Returns true if `name` is safe to embed in a shell command as a systemd
/// unit name. Only a conservative character set is accepted.
pub fn valid_unit_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | ':' | '-'))
}

fn unit(name: &str) -> String {
    // Callers must validate first; refuse to build anything otherwise.
    if !valid_unit_name(name) {
        return "echo '#invalid'".into();
    }
    let unit = if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.service")
    };
    format!(
        r##"if [ -d /run/systemd/system ] && command -v systemctl >/dev/null 2>&1; then
  echo "#systemd"
  systemctl show --no-pager -p Id -p LoadState -p ActiveState -p SubState -p UnitFileState -p Description -- '{unit}' 2>/dev/null
  echo "#rc $?"
fi"##
    )
}

/// Marker line for a section.
pub fn marker(nonce: &str, name: &str) -> String {
    format!("@@SKRY-{nonce}:{name}@@")
}

/// Generates a fresh nonce for one invocation.
pub fn new_nonce() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default(),
    );
    format!("{:016x}", h.finish())
}

/// The POSIX sh script for the given sections, before quoting.
pub fn script_body(sections: &[Section], nonce: &str) -> String {
    let mut s = String::with_capacity(4096);
    s.push_str("LC_ALL=C; export LC_ALL\n");
    s.push_str("PATH=\"$PATH:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"; export PATH\n");
    s.push_str("T=''; if command -v timeout >/dev/null 2>&1; then T='timeout 10'; fi\n");
    let _ = writeln!(s, "m() {{ printf '\\n@@SKRY-{nonce}:%s@@\\n' \"$1\"; }}");
    for section in sections {
        let _ = writeln!(s, "m {}", section.name());
        s.push_str(&section.body());
        s.push('\n');
    }
    s.push_str("m end\n");
    s
}

/// Builds the complete remote command for the given sections.
pub fn build(sections: &[Section], nonce: &str) -> String {
    // Wrap in `sh -c` so that the remote login shell (which may be bash,
    // zsh, fish, ...) does not matter: the script always runs under POSIX sh.
    format!("sh -c {}", shell_quote(&script_body(sections, nonce)))
}

/// Single-quotes a string for POSIX sh.
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_use_nonce() {
        let cmd = build(&[Section::Uptime], "abc123");
        assert!(cmd.starts_with("sh -c '"));
        assert!(cmd.contains("@@SKRY-abc123:%s@@"));
        assert!(cmd.contains("m uptime"));
        assert!(cmd.contains("m end"));
    }

    #[test]
    fn nonces_differ() {
        assert_ne!(new_nonce(), new_nonce());
    }

    #[test]
    fn quote_roundtrip() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn unit_names_are_validated() {
        assert!(valid_unit_name("nginx"));
        assert!(valid_unit_name("getty@tty1.service"));
        assert!(!valid_unit_name("x; rm -rf /"));
        assert!(!valid_unit_name("$(id)"));
        assert!(!valid_unit_name("a'b"));
        assert!(!valid_unit_name("-p"));
        assert!(!valid_unit_name(""));
        assert_eq!(unit("bad name"), "echo '#invalid'");
    }

    #[test]
    fn no_bashisms() {
        let cmd = build(&Section::all(true), "n");
        for bad in ["[[", "function ", "<(", "$'", "&>", "declare ", "local "] {
            assert!(!cmd.contains(bad), "found bashism {bad:?}");
        }
    }

    #[test]
    fn rpm_updates_are_opt_in() {
        assert!(!updates(false).contains("dnf -C"));
        assert!(updates(true).contains("dnf -C"));
    }
}
