//! Small formatting helpers shared by the TUI, web and reports.

/// Formats a byte count with binary units: `1.5 GiB`, `320 KiB`, `12 B`.
pub fn human_bytes(v: f64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if !v.is_finite() || v < 0.0 {
        return "n/a".into();
    }
    let mut v = v;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 || v >= 100.0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Formats a duration in seconds as `3d 4h`, `5h 12m`, `7m 3s` or `42s`.
pub fn human_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "n/a".into();
    }
    let s = secs as u64;
    let (d, h, m, s) = (s / 86400, s % 86400 / 3600, s % 3600 / 60, s % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// `12.3%` or `n/a`.
pub fn pct(v: Option<f64>) -> String {
    v.map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "n/a".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes() {
        assert_eq!(human_bytes(0.0), "0 B");
        assert_eq!(human_bytes(1023.0), "1023 B");
        assert_eq!(human_bytes(1536.0), "1.5 KiB");
        assert_eq!(human_bytes(250.0 * 1024.0 * 1024.0), "250 MiB");
        assert_eq!(human_bytes(-1.0), "n/a");
    }

    #[test]
    fn durations() {
        assert_eq!(human_duration(42.0), "42s");
        assert_eq!(human_duration(423.0), "7m 3s");
        assert_eq!(human_duration(5.0 * 3600.0 + 720.0), "5h 12m");
        assert_eq!(human_duration(3.0 * 86400.0 + 4.0 * 3600.0), "3d 4h");
    }

    #[test]
    fn percentages() {
        assert_eq!(pct(Some(12.345)), "12.3%");
        assert_eq!(pct(None), "n/a");
    }
}
