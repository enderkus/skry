//! Pure parsers for the remote command output. No I/O happens here, which
//! keeps every parser testable against the fixture files in
//! `tests/fixtures`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Datelike, NaiveDate, NaiveDateTime};

use super::script::marker;
use super::types::*;

/// The remote output split into sections.
#[derive(Debug, Default, Clone)]
pub struct Sections {
    map: BTreeMap<String, String>,
    /// True when the terminating marker was seen (output not truncated).
    pub complete: bool,
}

impl Sections {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.map.get(name).map(String::as_str)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
}

/// Splits raw output on the nonce-protected marker lines.
pub fn split_sections(output: &str, nonce: &str) -> Sections {
    let prefix = marker(nonce, "");
    let prefix = prefix.trim_end_matches("@@");
    let mut sections = Sections::default();
    let mut current: Option<String> = None;
    let mut buf = String::new();
    for line in output.lines() {
        let trimmed = line.trim_end_matches('\r');
        if let Some(name) = trimmed
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix("@@"))
        {
            if let Some(cur) = current.take() {
                sections.map.insert(cur, std::mem::take(&mut buf));
            }
            buf.clear();
            if name == "end" {
                sections.complete = true;
            } else {
                current = Some(name.to_string());
            }
            continue;
        }
        if current.is_some() {
            buf.push_str(trimmed);
            buf.push('\n');
        }
    }
    if let Some(cur) = current.take() {
        sections.map.insert(cur, buf);
    }
    sections
}

fn num<T: std::str::FromStr>(s: &str) -> Option<T> {
    s.trim().parse().ok()
}

fn meaningful_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// Splits `line` on whitespace into at most `n` parts; the last part keeps
/// its inner whitespace.
fn split_ws_n(line: &str, n: usize) -> Vec<&str> {
    let mut out = Vec::with_capacity(n);
    let mut rest = line.trim_start();
    while out.len() + 1 < n {
        match rest.find(char::is_whitespace) {
            Some(i) => {
                out.push(&rest[..i]);
                rest = rest[i..].trim_start();
            }
            None => break,
        }
    }
    if !rest.is_empty() {
        out.push(rest.trim_end());
    }
    out
}

pub fn parse_meta(text: &str) -> Identity {
    let mut id = Identity {
        page_size: 4096,
        clk_tck: 100,
        ..Default::default()
    };
    for line in meaningful_lines(text) {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k {
            "hostname" => id.hostname = v.to_string(),
            "kernel" => id.kernel = v.to_string(),
            "arch" => id.arch = v.to_string(),
            "time" => id.remote_time = num(v),
            "localtime" if !v.is_empty() => id.local_hour = Some(v.to_string()),
            "pagesize" => id.page_size = num(v).filter(|&p: &u64| p > 0).unwrap_or(4096),
            "clk_tck" => id.clk_tck = num(v).filter(|&c: &u64| c > 0).unwrap_or(100),
            "uid" => id.uid = num(v),
            _ => {}
        }
    }
    id
}

pub fn parse_os_release(text: &str) -> Option<OsRelease> {
    let mut os = OsRelease::default();
    let mut any = false;
    for line in meaningful_lines(text) {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
        any = true;
        match k {
            "ID" => os.id = v,
            "ID_LIKE" => os.id_like = v,
            "NAME" => os.name = v,
            "PRETTY_NAME" => os.pretty_name = v,
            "VERSION_ID" => os.version_id = v,
            _ => {}
        }
    }
    if os.pretty_name.is_empty() {
        os.pretty_name = format!("{} {}", os.name, os.version_id).trim().to_string();
    }
    any.then_some(os)
}

pub fn parse_cpuinfo(text: &str) -> Option<String> {
    meaningful_lines(text)
        .next()
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
}

fn cpu_counters(fields: &[&str]) -> CpuCounters {
    let f = |i: usize| fields.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
    CpuCounters {
        user: f(0),
        nice: f(1),
        system: f(2),
        idle: f(3),
        iowait: f(4),
        irq: f(5),
        softirq: f(6),
        steal: f(7),
    }
}

pub fn parse_stat(text: &str) -> Option<CpuStat> {
    let mut stat = CpuStat::default();
    let mut seen_total = false;
    for line in meaningful_lines(text) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some((&key, rest)) = fields.split_first() else {
            continue;
        };
        match key {
            "cpu" => {
                stat.total = cpu_counters(rest);
                seen_total = true;
            }
            k if k.starts_with("cpu") => stat.cores.push(cpu_counters(rest)),
            "btime" => stat.btime = rest.first().and_then(|v| num(v)),
            "procs_running" => stat.procs_running = rest.first().and_then(|v| num(v)),
            "procs_blocked" => stat.procs_blocked = rest.first().and_then(|v| num(v)),
            _ => {}
        }
    }
    seen_total.then_some(stat)
}

pub fn parse_meminfo(text: &str) -> Option<MemInfo> {
    let mut values: HashMap<&str, u64> = HashMap::new();
    for line in meaningful_lines(text) {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let mut parts = v.split_whitespace();
        let Some(n) = parts.next().and_then(|n| n.parse::<u64>().ok()) else {
            continue;
        };
        let mult = match parts.next() {
            Some("kB") => 1024,
            _ => 1,
        };
        values.insert(k.trim(), n * mult);
    }
    let total = *values.get("MemTotal")?;
    let g = |k: &str| values.get(k).copied().unwrap_or(0);
    let available = values.get("MemAvailable").copied().unwrap_or_else(|| {
        // Kernels before 3.14 lack MemAvailable; approximate it.
        g("MemFree") + g("Buffers") + g("Cached") + g("SReclaimable")
    });
    Some(MemInfo {
        total,
        free: g("MemFree"),
        available: available.min(total),
        buffers: g("Buffers"),
        cached: g("Cached"),
        sreclaimable: g("SReclaimable"),
        swap_total: g("SwapTotal"),
        swap_free: g("SwapFree"),
    })
}

pub fn parse_loadavg(text: &str) -> Option<LoadAvg> {
    let line = meaningful_lines(text).next()?;
    let f: Vec<&str> = line.split_whitespace().collect();
    let (running, total) = f
        .get(3)
        .and_then(|s| s.split_once('/'))
        .map(|(r, t)| (num(r).unwrap_or(0), num(t).unwrap_or(0)))
        .unwrap_or((0, 0));
    Some(LoadAvg {
        one: num(f.first()?)?,
        five: num(f.get(1)?)?,
        fifteen: num(f.get(2)?)?,
        running,
        total,
    })
}

pub fn parse_uptime(text: &str) -> Option<f64> {
    meaningful_lines(text)
        .next()?
        .split_whitespace()
        .next()
        .and_then(num)
}

pub fn parse_netdev(text: &str) -> Vec<NetDevCounters> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || name.contains('|') {
            continue;
        }
        let v: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|x| x.parse().ok())
            .collect();
        if v.len() < 16 {
            continue;
        }
        out.push(NetDevCounters {
            name: name.to_string(),
            rx_bytes: v[0],
            rx_packets: v[1],
            rx_errs: v[2],
            rx_drop: v[3],
            tx_bytes: v[8],
            tx_packets: v[9],
            tx_errs: v[10],
            tx_drop: v[11],
        });
    }
    out
}

pub fn parse_diskstats(text: &str) -> Vec<DiskStatCounters> {
    let mut out = Vec::new();
    for line in meaningful_lines(text) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 14 {
            continue;
        }
        let n = |i: usize| f[i].parse::<u64>().unwrap_or(0);
        out.push(DiskStatCounters {
            major: f[0].parse().unwrap_or(0),
            minor: f[1].parse().unwrap_or(0),
            name: f[2].to_string(),
            reads: n(3),
            sectors_read: n(5),
            writes: n(7),
            sectors_written: n(9),
            io_ms: n(12),
        });
    }
    out
}

pub fn parse_df(text: &str) -> Vec<DfEntry> {
    let mut out = Vec::new();
    for line in meaningful_lines(text) {
        let f: Vec<&str> = line.split_whitespace().collect();
        // Locate the capacity column ("42%") and work outwards from it so
        // that spaces in the device or mount point do not break parsing.
        let Some(i) = f
            .iter()
            .position(|t| t.ends_with('%') && t[..t.len() - 1].parse::<u64>().is_ok())
        else {
            continue;
        };
        if i < 4 || i + 1 >= f.len() {
            continue;
        }
        let (Some(total), Some(used), Some(avail)) = (num(f[i - 3]), num(f[i - 2]), num(f[i - 1]))
        else {
            continue;
        };
        out.push(DfEntry {
            filesystem: f[..i - 3].join(" "),
            total_kb: total,
            used_kb: used,
            avail_kb: avail,
            mount: f[i + 1..].join(" "),
        });
    }
    out
}

pub fn parse_ipaddr(text: &str) -> Vec<IpAddrEntry> {
    let mut out = Vec::new();
    for line in meaningful_lines(text) {
        let line = line.split('\\').next().unwrap_or(line);
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 4 || !matches!(f[2], "inet" | "inet6") {
            continue;
        }
        let iface = f[1].split('@').next().unwrap_or(f[1]).to_string();
        let scope = f
            .iter()
            .position(|&t| t == "scope")
            .and_then(|i| f.get(i + 1))
            .copied()
            .unwrap_or("")
            .to_string();
        out.push(IpAddrEntry {
            iface,
            family: f[2].to_string(),
            cidr: f[3].to_string(),
            scope,
        });
    }
    out
}

pub fn parse_procstat(text: &str) -> Vec<ProcStat> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f = split_ws_n(line, 6);
        if f.len() < 5 {
            continue;
        }
        let Some(pid) = num(f[0]) else { continue };
        out.push(ProcStat {
            pid,
            state: f[1].to_string(),
            utime: num(f[2]).unwrap_or(0),
            stime: num(f[3]).unwrap_or(0),
            rss_pages: num(f[4]).unwrap_or(0),
            comm: f.get(5).map(|s| s.to_string()).unwrap_or_default(),
        });
    }
    out
}

pub fn parse_ps_users(text: &str) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        if let (Some(Ok(pid)), Some(user)) = (f.next().map(str::parse::<u32>), f.next()) {
            out.insert(pid, user.to_string());
        }
    }
    out
}

pub fn parse_ps_args(text: &str) -> Vec<ProcArgs> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f = split_ws_n(line, 3);
        if f.len() < 3 {
            continue;
        }
        let Some(pid) = num(f[0]) else { continue };
        out.push(ProcArgs {
            pid,
            user: f[1].to_string(),
            args: f[2].to_string(),
        });
    }
    out
}

fn split_host_port(local: &str) -> Option<(String, u16)> {
    let (addr, port) = local.rsplit_once(':')?;
    let port = port.parse().ok()?;
    let addr = addr.trim_start_matches('[').trim_end_matches(']');
    let addr = if addr.is_empty() || addr == "::" && local.starts_with(":::") {
        "::".to_string()
    } else {
        addr.to_string()
    };
    Some((addr, port))
}

fn decode_proc_net_addr(hex: &str) -> Option<(String, u16)> {
    let (addr, port) = hex.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    let addr = match addr.len() {
        8 => {
            let v = u32::from_str_radix(addr, 16).ok()?;
            std::net::Ipv4Addr::from(v.swap_bytes()).to_string()
        }
        32 => {
            let mut bytes = [0u8; 16];
            for (i, chunk) in addr.as_bytes().chunks(8).enumerate() {
                let word = u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
                bytes[i * 4..i * 4 + 4].copy_from_slice(&word.swap_bytes().to_be_bytes());
            }
            std::net::Ipv6Addr::from(bytes).to_string()
        }
        _ => return None,
    };
    Some((addr, port))
}

/// Parses `ss -tln[H]`, `netstat -tln` or raw `/proc/net/tcp{,6}` output.
pub fn parse_ports(text: &str) -> Probe<Vec<ListenPort>> {
    let mut set = BTreeSet::new();
    for line in meaningful_lines(text) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 4 {
            continue;
        }
        let is_ss = f[0] == "LISTEN";
        let is_netstat = f[0].starts_with("tcp") && f.contains(&"LISTEN");
        let parsed = if is_ss || is_netstat {
            split_host_port(f[3])
        } else if f[0].ends_with(':') && f[0][..f[0].len() - 1].parse::<u32>().is_ok() {
            // /proc/net/tcp: state 0A is TCP_LISTEN.
            if f[3] == "0A" {
                decode_proc_net_addr(f[1])
            } else {
                None
            }
        } else {
            None
        };
        if let Some((addr, port)) = parsed {
            let addr = addr.split('%').next().unwrap_or(&addr).to_string();
            set.insert(ListenPort { port, addr });
        }
    }
    Probe::Ok(set.into_iter().collect())
}

fn pct(s: &str) -> Option<f64> {
    s.trim().trim_end_matches('%').parse().ok()
}

pub fn parse_containers(text: &str) -> Probe<Vec<Container>> {
    let mut runtimes = Vec::new();
    let mut errors = Vec::new();
    let mut ok_runtimes = BTreeSet::new();
    let mut list: Vec<Container> = Vec::new();
    let mut runtime = String::new();
    for line in meaningful_lines(text) {
        if let Some(r) = line.strip_prefix("#runtime ") {
            runtime = r.trim().to_string();
            runtimes.push(runtime.clone());
            ok_runtimes.insert(runtime.clone());
        } else if let Some(e) = line.strip_prefix("#error") {
            ok_runtimes.remove(&runtime);
            errors.push(format!("{runtime}: {}", e.trim()));
        } else if let Some(rest) = line.strip_prefix("c|") {
            let p: Vec<&str> = rest.split('|').collect();
            if p.len() < 3 {
                continue;
            }
            list.push(Container {
                runtime: runtime.clone(),
                name: p[0].to_string(),
                image: p[1].to_string(),
                status: p[2].to_string(),
                state: p.get(3).unwrap_or(&"").to_string(),
                ..Default::default()
            });
        } else if let Some(rest) = line.strip_prefix("s|") {
            let p: Vec<&str> = rest.split('|').collect();
            if p.len() < 4 {
                continue;
            }
            if let Some(c) = list
                .iter_mut()
                .find(|c| c.runtime == runtime && c.name == p[0])
            {
                c.cpu_pct = pct(p[1]);
                c.mem_usage = Some(p[2].trim().to_string()).filter(|s| s != "--" && !s.is_empty());
                c.mem_pct = pct(p[3]);
            }
        }
    }
    if runtimes.is_empty() {
        return Probe::na("no docker or podman");
    }
    if ok_runtimes.is_empty() {
        return Probe::na(simplify_error(&errors.join("; ")));
    }
    Probe::Ok(list)
}

fn simplify_error(e: &str) -> String {
    if e.contains("permission denied") || e.contains("Permission denied") {
        let rt = e.split(':').next().unwrap_or("runtime");
        format!("{rt}: permission denied")
    } else {
        e.chars().take(120).collect()
    }
}

fn systemd_rc(text: &str) -> Option<i32> {
    meaningful_lines(text)
        .filter_map(|l| l.strip_prefix("#rc "))
        .last()
        .and_then(num)
}

pub fn parse_failed_units(text: &str) -> Probe<Vec<FailedUnit>> {
    if !text.lines().any(|l| l.trim() == "#systemd") {
        return Probe::na("no systemd");
    }
    match systemd_rc(text) {
        Some(0) | None => {}
        Some(_) => return Probe::na("systemctl not permitted"),
    }
    let mut out = Vec::new();
    for line in meaningful_lines(text) {
        if line.starts_with('#') {
            continue;
        }
        let line = line.trim_start_matches(['●', '*', '×']).trim_start();
        let f = split_ws_n(line, 5);
        if f.len() < 4 {
            continue;
        }
        out.push(FailedUnit {
            unit: f[0].to_string(),
            load: f[1].to_string(),
            active: f[2].to_string(),
            sub: f[3].to_string(),
            description: f.get(4).unwrap_or(&"").to_string(),
        });
    }
    Probe::Ok(out)
}

pub fn parse_unit_status(text: &str) -> Probe<UnitStatus> {
    if text.lines().any(|l| l.trim() == "#invalid") {
        return Probe::na("invalid unit name");
    }
    if !text.lines().any(|l| l.trim() == "#systemd") {
        return Probe::na("no systemd");
    }
    let mut u = UnitStatus::default();
    for line in meaningful_lines(text) {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.to_string();
        match k {
            "Id" => u.id = v,
            "LoadState" => u.load_state = v,
            "ActiveState" => u.active_state = v,
            "SubState" => u.sub_state = v,
            "UnitFileState" => u.unit_file_state = v,
            "Description" => u.description = v,
            _ => {}
        }
    }
    if u.load_state.is_empty() {
        return match systemd_rc(text) {
            Some(rc) if rc != 0 => Probe::na("systemctl not permitted"),
            _ => Probe::na("no data"),
        };
    }
    Probe::Ok(u)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn parse_hour_key(key: &str, now: NaiveDateTime) -> Option<NaiveDateTime> {
    if key.len() == 13 && key.as_bytes()[4] == b'-' {
        return NaiveDateTime::parse_from_str(&format!("{key}:00:00"), "%Y-%m-%dT%H:%M:%S").ok();
    }
    let f: Vec<&str> = key.split_whitespace().collect();
    if f.len() != 3 {
        return None;
    }
    let month = MONTHS.iter().position(|&m| m == f[0])? as u32 + 1;
    let day: u32 = f[1].parse().ok()?;
    let hour: u32 = f[2].parse().ok()?;
    let mk = |y: i32| NaiveDate::from_ymd_opt(y, month, day)?.and_hms_opt(hour, 0, 0);
    let t = mk(now.year())?;
    // Syslog timestamps carry no year; a date in the future belongs to last year.
    if t > now + chrono::Duration::hours(1) {
        mk(now.year() - 1)
    } else {
        Some(t)
    }
}

/// Parses the aggregated failed-login summary. `local_hour` is the remote
/// local time (`YYYY-MM-DDTHH`) used to apply the 24-hour window to log
/// files; journal output is already windowed with `--since`.
pub fn parse_failed_logins(text: &str, local_hour: Option<&str>) -> Probe<FailedLogins> {
    let mut source = None;
    let mut denied = false;
    let mut entries: Vec<(u64, String, String)> = Vec::new();
    for line in meaningful_lines(text) {
        if let Some(s) = line.strip_prefix("#source ") {
            source = Some(s.trim().to_string());
        } else if line == "#denied" {
            denied = true;
        } else if let Some(rest) = line.strip_prefix("k ") {
            let f = split_ws_n(rest, 3);
            if f.len() == 3
                && let Some(n) = num::<u64>(f[0])
            {
                entries.push((n, f[1].to_string(), f[2].to_string()));
            }
        }
    }
    let Some(source) = source.or_else(|| denied.then(String::new)) else {
        return Probe::na("no journal or auth log");
    };
    if denied && entries.is_empty() {
        return Probe::na("logs not readable (needs adm or systemd-journal group)");
    }
    let now = local_hour.and_then(|h| {
        NaiveDateTime::parse_from_str(&format!("{h}:00:00"), "%Y-%m-%dT%H:%M:%S").ok()
    });
    let windowed = source != "journal";
    let mut by_ip: HashMap<String, u64> = HashMap::new();
    let mut total = 0;
    for (n, ip, key) in entries {
        if windowed && let Some(now) = now {
            match parse_hour_key(&key, now) {
                Some(t) => {
                    let age = now - t;
                    if age < chrono::Duration::zero() || age >= chrono::Duration::hours(24) {
                        continue;
                    }
                }
                None => continue,
            }
        }
        total += n;
        *by_ip.entry(ip).or_default() += n;
    }
    let mut top: Vec<(String, u64)> = by_ip.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top.truncate(10);
    Probe::Ok(FailedLogins {
        source,
        total,
        top_sources: top,
    })
}

/// Strips `-version-release` (and `.arch` for RPM) from a package string.
fn package_name(nevra: &str, strip_arch: bool) -> String {
    let base = if strip_arch {
        nevra.rsplit_once('.').map(|(b, _)| b).unwrap_or(nevra)
    } else {
        nevra
    };
    let mut parts = base.rsplitn(3, '-');
    let _release = parts.next();
    let _version = parts.next();
    parts.next().unwrap_or(base).to_string()
}

pub fn parse_updates(text: &str) -> Probe<Updates> {
    let mut manager = None;
    for line in meaningful_lines(text) {
        match line {
            "#denied" => return Probe::na("requires root on RPM systems"),
            "#disabled" => return Probe::na("disabled on RPM systems; set security.rpm_updates"),
            l => {
                if let Some(m) = l.strip_prefix("#manager ") {
                    manager = Some(m.trim().to_string());
                }
            }
        }
    }
    let Some(manager) = manager else {
        return Probe::na("no supported package manager");
    };
    let mut all = BTreeSet::new();
    let mut security = BTreeSet::new();
    for line in meaningful_lines(text).filter(|l| !l.starts_with('#')) {
        match manager.as_str() {
            "apt" => {
                let Some(rest) = line.strip_prefix("Inst ") else {
                    continue;
                };
                let name = rest.split_whitespace().next().unwrap_or("").to_string();
                let origin = rest.split_once('(').map(|(_, o)| o).unwrap_or("");
                if origin.contains("-security") || origin.contains("Debian-Security") {
                    security.insert(name.clone());
                }
                all.insert(name);
            }
            "apk" => {
                if let Some(pkg) = line.split_whitespace().next()
                    && line.contains('<')
                {
                    all.insert(package_name(pkg, false));
                }
            }
            "dnf" => {
                let f: Vec<&str> = line.split_whitespace().collect();
                if f.len() < 3 {
                    continue;
                }
                let name = package_name(f[2], true);
                if f[1].ends_with("/Sec.") || f[1] == "security" {
                    security.insert(name.clone());
                }
                all.insert(name);
            }
            _ => {}
        }
    }
    let has_security_metadata = manager != "apk";
    Probe::Ok(Updates {
        manager,
        total: all.len() as u64,
        security: has_security_metadata.then_some(security.len() as u64),
        security_packages: security.into_iter().collect(),
    })
}

/// Everything parsed from one remote invocation. Sections that were not
/// part of the command are `None`.
#[derive(Debug, Clone, Default)]
pub struct RawSample {
    pub complete: bool,
    pub identity: Option<Identity>,
    pub os: Option<OsRelease>,
    pub cpu_model: Option<Option<String>>,
    pub stat: Option<CpuStat>,
    pub mem: Option<MemInfo>,
    pub load: Option<LoadAvg>,
    pub uptime: Option<f64>,
    pub net: Vec<NetDevCounters>,
    pub diskstats: Vec<DiskStatCounters>,
    pub df: Vec<DfEntry>,
    pub ips: Option<Vec<IpAddrEntry>>,
    pub procs: Vec<ProcStat>,
    pub users: HashMap<u32, String>,
    pub ps_args: Option<Vec<ProcArgs>>,
    pub ports: Option<Probe<Vec<ListenPort>>>,
    pub containers: Option<Probe<Vec<Container>>>,
    pub failed_units: Option<Probe<Vec<FailedUnit>>>,
    pub failed_logins: Option<Probe<FailedLogins>>,
    pub updates: Option<Probe<Updates>>,
    pub unit: Option<Probe<UnitStatus>>,
}

impl RawSample {
    pub fn parse(output: &str, nonce: &str) -> RawSample {
        let s = split_sections(output, nonce);
        let identity = s.get("meta").map(parse_meta);
        let local_hour = identity.as_ref().and_then(|i| i.local_hour.clone());
        RawSample {
            complete: s.complete,
            os: s.get("os").and_then(parse_os_release),
            cpu_model: s.get("cpuinfo").map(parse_cpuinfo),
            stat: s.get("stat").and_then(parse_stat),
            mem: s.get("meminfo").and_then(parse_meminfo),
            load: s.get("loadavg").and_then(parse_loadavg),
            uptime: s.get("uptime").and_then(parse_uptime),
            net: s.get("netdev").map(parse_netdev).unwrap_or_default(),
            diskstats: s.get("diskstats").map(parse_diskstats).unwrap_or_default(),
            df: s.get("df").map(parse_df).unwrap_or_default(),
            ips: s.get("ipaddr").map(parse_ipaddr),
            procs: s.get("procstat").map(parse_procstat).unwrap_or_default(),
            users: s.get("ps").map(parse_ps_users).unwrap_or_default(),
            ps_args: s.get("psargs").map(parse_ps_args),
            ports: s.get("ports").map(parse_ports),
            containers: s.get("containers").map(parse_containers),
            failed_units: s.get("units").map(parse_failed_units),
            failed_logins: s
                .get("logins")
                .map(|t| parse_failed_logins(t, local_hour.as_deref())),
            updates: s.get("updates").map(parse_updates),
            unit: s.get("unit").map(parse_unit_status),
            identity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: &str = "0123456789abcdef";

    macro_rules! fixture {
        ($path:literal) => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/",
                $path
            ))
        };
    }

    fn section<'a>(s: &'a Sections, name: &str) -> &'a str {
        s.get(name)
            .unwrap_or_else(|| panic!("missing section {name}"))
    }

    #[test]
    fn split_ignores_forged_markers() {
        let out = "@@SKRY-0123456789abcdef:meta@@\nhostname=a\n@@SKRY-deadbeef:end@@\n(@@SKRY-0123456789abcdef:end@@)\n";
        let s = split_sections(out, NONCE);
        assert!(!s.complete);
        assert_eq!(s.get("meta").unwrap().lines().count(), 3);
    }

    #[test]
    fn truncated_output_keeps_sections() {
        let out = "@@SKRY-0123456789abcdef:uptime@@\n12.5 40.0\n@@SKRY-0123456789abcdef:loadavg@@\n0.1 0.2";
        let s = split_sections(out, NONCE);
        assert!(!s.complete);
        assert_eq!(parse_uptime(s.get("uptime").unwrap()), Some(12.5));
        assert!(s.get("loadavg").is_some());
    }

    #[test]
    fn debian_full() {
        let r = RawSample::parse(fixture!("debian/full.txt"), NONCE);
        assert!(r.complete);
        let os = r.os.unwrap();
        assert_eq!(os.id, "debian");
        assert_eq!(os.pretty_name, "Debian GNU/Linux 12 (bookworm)");
        let id = r.identity.unwrap();
        assert!(!id.hostname.is_empty());
        assert_eq!(id.page_size, 4096);
        assert_eq!(id.uid, Some(1000));
        let stat = r.stat.unwrap();
        assert!(!stat.cores.is_empty());
        assert!(stat.total.total() > 0);
        let mem = r.mem.unwrap();
        assert!(mem.total > mem.available);
        assert!(r.load.is_some());
        assert!(r.uptime.unwrap() > 0.0);
        assert!(r.net.iter().any(|n| n.name == "eth0" && n.rx_bytes > 0));
        assert!(r.diskstats.iter().any(|d| d.name == "vda"));
        assert!(r.df.iter().any(|d| d.mount == "/"));
        assert!(r.procs.iter().any(|p| p.comm == "sshd"));
        assert!(r.users.values().any(|u| u == "skry"));
        let ports = r.ports.unwrap();
        assert!(
            ports
                .ok()
                .unwrap()
                .iter()
                .any(|p| p.port == 22 && p.addr == "0.0.0.0")
        );
        assert!(
            ports
                .ok()
                .unwrap()
                .iter()
                .any(|p| p.port == 22 && p.addr == "::")
        );
        assert_eq!(r.containers.unwrap(), Probe::na("no docker or podman"));
        assert_eq!(r.failed_units.unwrap(), Probe::na("no systemd"));
        let ips = r.ips.unwrap();
        assert!(ips.iter().any(|i| i.iface == "eth0" && i.family == "inet"));
        let updates = r.updates.unwrap();
        assert_eq!(updates.ok().unwrap().manager, "apt");
    }

    #[test]
    fn ubuntu_full() {
        let r = RawSample::parse(fixture!("ubuntu/full.txt"), NONCE);
        assert!(r.complete);
        assert_eq!(r.os.unwrap().id, "ubuntu");
        let u = r.updates.unwrap();
        let u = u.ok().unwrap();
        assert_eq!(u.manager, "apt");
        assert!(u.total >= u.security.unwrap());
    }

    #[test]
    fn rocky_full() {
        let r = RawSample::parse(fixture!("rocky/full.txt"), NONCE);
        assert!(r.complete);
        let os = r.os.unwrap();
        assert_eq!(os.id, "rocky");
        assert_eq!(os.id_like, "rhel centos fedora");
        assert_eq!(
            r.updates.unwrap(),
            Probe::na("requires root on RPM systems")
        );
        assert!(r.procs.iter().any(|p| p.comm == "sshd"));
    }

    #[test]
    fn alpine_busybox_full() {
        let r = RawSample::parse(fixture!("alpine/full.txt"), NONCE);
        assert!(r.complete);
        assert_eq!(r.os.unwrap().id, "alpine");
        // BusyBox netstat output.
        let ports = r.ports.unwrap();
        let ports = ports.ok().unwrap();
        assert!(ports.iter().any(|p| p.port == 22 && p.addr == "0.0.0.0"));
        assert!(ports.iter().any(|p| p.port == 22 && p.addr == "::"));
        assert!(!r.procs.is_empty());
        assert!(!r.users.is_empty());
        assert!(r.df.iter().any(|d| d.mount == "/"));
    }

    #[test]
    fn systemd_root() {
        let r = RawSample::parse(fixture!("debian-systemd/full-root.txt"), NONCE);
        let units = r.failed_units.unwrap();
        let units = units.ok().unwrap();
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].unit, "backup-nightly.service");
        assert_eq!(units[0].description, "Nightly backup job");
        assert_eq!(units[1].active, "failed");
        let logins = r.failed_logins.unwrap();
        let logins = logins.ok().unwrap();
        assert_eq!(logins.source, "journal");
        assert_eq!(logins.total, 9);
        assert_eq!(logins.top_sources[0], ("127.0.0.1".to_string(), 7));
        let updates = r.updates.unwrap();
        let updates = updates.ok().unwrap();
        assert_eq!(updates.security, Some(1));
        assert_eq!(updates.security_packages, vec!["tzdata".to_string()]);
    }

    #[test]
    fn systemd_unprivileged_is_not_a_false_all_clear() {
        let r = RawSample::parse(fixture!("debian-systemd/full-user.txt"), NONCE);
        assert_eq!(
            r.failed_units.unwrap(),
            Probe::na("systemctl not permitted")
        );
        assert!(matches!(r.failed_logins.unwrap(), Probe::Unavailable(_)));
    }

    #[test]
    fn auth_log_iso_timestamps() {
        let p = parse_failed_logins(
            fixture!("debian-systemd/logins-authlog.txt"),
            Some("2026-09-29T01"),
        );
        let p = p.ok().unwrap();
        assert_eq!(p.source, "/var/log/auth.log");
        assert_eq!(p.total, 9);
        // A day later everything has aged out of the window.
        let p = parse_failed_logins(
            fixture!("debian-systemd/logins-authlog.txt"),
            Some("2026-09-30T02"),
        );
        assert_eq!(p.ok().unwrap().total, 0);
    }

    #[test]
    fn syslog_traditional_timestamps() {
        let text = fixture!("alpine/logins-messages.txt");
        let p = parse_failed_logins(text, Some("2026-09-29T05"));
        assert_eq!(p.ok().unwrap().total, 3);
        // Year rollover: a December entry seen in early January.
        let text = "#source /var/log/secure\nk 4 10.0.0.9 Dec 31 23\nk 1 10.0.0.8 Dec 30 01\n";
        let p = parse_failed_logins(text, Some("2027-01-01T03"));
        let p = p.ok().unwrap();
        assert_eq!(p.total, 4);
        assert_eq!(p.top_sources, vec![("10.0.0.9".to_string(), 4)]);
    }

    #[test]
    fn logins_unavailable() {
        assert_eq!(
            parse_failed_logins("", None),
            Probe::na("no journal or auth log")
        );
        assert!(matches!(
            parse_failed_logins("#denied\n", None),
            Probe::Unavailable(_)
        ));
    }

    #[test]
    fn apk_updates() {
        let u = parse_updates(fixture!("alpine/updates-apk.txt"));
        let u = u.ok().unwrap();
        assert_eq!(u.manager, "apk");
        assert_eq!(u.total, 10);
        assert_eq!(u.security, None);
    }

    #[test]
    fn dnf_updates() {
        let u = parse_updates(fixture!("rocky/updates-dnf.txt"));
        let u = u.ok().unwrap();
        assert_eq!(u.manager, "dnf");
        assert!(u.total > 50);
        let sec = u.security.unwrap();
        assert!(sec > 0 && sec < u.total);
        assert!(u.security_packages.contains(&"curl-minimal".to_string()));
        assert!(u.security_packages.contains(&"binutils".to_string()));
        assert!(!u.security_packages.contains(&"bash".to_string()));
    }

    #[test]
    fn updates_na() {
        assert!(matches!(
            parse_updates("#disabled\n"),
            Probe::Unavailable(_)
        ));
        assert_eq!(parse_updates(""), Probe::na("no supported package manager"));
    }

    #[test]
    fn docker_containers() {
        let c = parse_containers(fixture!("common/containers-docker.txt"));
        let c = c.ok().unwrap();
        let debian = c.iter().find(|c| c.name == "skry-it-debian-1").unwrap();
        assert_eq!(debian.runtime, "docker");
        assert_eq!(debian.state, "running");
        assert_eq!(debian.cpu_pct, Some(0.0));
        assert_eq!(debian.mem_usage.as_deref(), Some("14MiB / 7.817GiB"));
        let exited = c.iter().find(|c| c.name == "skry-fx-exited").unwrap();
        assert_eq!(exited.state, "exited");
        assert_eq!(exited.cpu_pct, None);
    }

    #[test]
    fn docker_permission_denied() {
        let text = "#runtime docker\n#error permission denied while trying to connect to the Docker daemon socket at unix:///var/run/docker.sock\n";
        assert_eq!(
            parse_containers(text),
            Probe::na("docker: permission denied")
        );
        assert_eq!(parse_containers(""), Probe::na("no docker or podman"));
    }

    #[test]
    fn ports_from_proc_net_tcp() {
        let text = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 1 1 0 100 0 0 10 0\n   1: 0100007F:1F90 0100007F:D2A4 01 00000000:00000000 00:00000000 00000000     0        0 1\n   0: 00000000000000000000000000000000:0016 00000000000000000000000000000000:0000 0A 0 0 0\n";
        let p = parse_ports(text);
        assert_eq!(
            p.ok().unwrap(),
            &vec![
                ListenPort {
                    port: 22,
                    addr: "0.0.0.0".into()
                },
                ListenPort {
                    port: 22,
                    addr: "::".into()
                },
            ]
        );
    }

    #[test]
    fn ports_ss_variants() {
        let text = "State  Recv-Q Send-Q Local Address:Port Peer Address:Port\nLISTEN 0 4096 127.0.0.53%lo:53 0.0.0.0:*\nLISTEN 0 511 *:80 *:*\nLISTEN 0 128 [::ffff:127.0.0.1]:8080 *:*\n";
        let p = parse_ports(text);
        let p = p.ok().unwrap();
        assert!(p.contains(&ListenPort {
            port: 53,
            addr: "127.0.0.53".into()
        }));
        assert!(p.contains(&ListenPort {
            port: 80,
            addr: "*".into()
        }));
        assert!(p.contains(&ListenPort {
            port: 8080,
            addr: "::ffff:127.0.0.1".into()
        }));
    }

    #[test]
    fn df_with_spaces() {
        let text = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/sda1 100 40 60 40% /mnt/my disk\n";
        let d = parse_df(text);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].mount, "/mnt/my disk");
        assert_eq!(d[0].used_kb, 40);
    }

    #[test]
    fn meminfo_without_available() {
        let text = "MemTotal: 1000 kB\nMemFree: 100 kB\nBuffers: 50 kB\nCached: 200 kB\n";
        let m = parse_meminfo(text).unwrap();
        assert_eq!(m.available, 350 * 1024);
    }

    #[test]
    fn procstat_comm_with_spaces() {
        let p = parse_procstat("42 S 10 5 300 tmux: server\n");
        assert_eq!(p[0].comm, "tmux: server");
        assert_eq!(p[0].utime, 10);
    }

    #[test]
    fn unit_status() {
        let text = "#systemd\nId=nginx.service\nLoadState=loaded\nActiveState=active\nSubState=running\nUnitFileState=enabled\nDescription=A high performance web server\n#rc 0\n";
        let u = parse_unit_status(text);
        let u = u.ok().unwrap();
        assert_eq!(u.active_state, "active");
        assert_eq!(u.description, "A high performance web server");
        assert_eq!(
            parse_unit_status("#systemd\n#rc 1\n"),
            Probe::na("systemctl not permitted")
        );
        assert_eq!(parse_unit_status(""), Probe::na("no systemd"));
    }

    #[test]
    fn stat_on_old_kernel_without_steal() {
        let s = parse_stat("cpu 1 2 3 4\ncpu0 1 2 3 4\n").unwrap();
        assert_eq!(s.total.total(), 10);
        assert_eq!(s.cores.len(), 1);
    }

    #[test]
    fn empty_sections_are_none() {
        assert!(parse_stat("").is_none());
        assert!(parse_meminfo("").is_none());
        assert!(parse_loadavg("").is_none());
        assert!(parse_uptime("").is_none());
        assert!(parse_os_release("").is_none());
        assert!(parse_cpuinfo("").is_none());
    }

    #[test]
    fn busybox_ps_header_is_skipped() {
        let users = parse_ps_users("PID   USER\n    1 root\n  17 nobody\n");
        assert_eq!(users.len(), 2);
        assert_eq!(users[&17], "nobody");
    }

    #[test]
    fn every_fixture_section_parses() {
        for text in [
            fixture!("debian/full.txt"),
            fixture!("ubuntu/full.txt"),
            fixture!("rocky/full.txt"),
            fixture!("alpine/full.txt"),
            fixture!("debian-systemd/full-root.txt"),
        ] {
            let s = split_sections(text, NONCE);
            for name in [
                "meta",
                "stat",
                "meminfo",
                "loadavg",
                "uptime",
                "netdev",
                "diskstats",
                "df",
                "procstat",
            ] {
                assert!(!section(&s, name).trim().is_empty(), "{name} empty");
            }
        }
    }
}
