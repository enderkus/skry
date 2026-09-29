//! Rendering. Every function here is a pure function of the app state and
//! the data to show, so screens can be snapshot-tested with `TestBackend`.

use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
};

use super::app::{App, HostTab, Screen};
use crate::collect::Probe;
use crate::engine::{ConnState, FleetState, HostState, HostStatus};
use crate::model::{Band, HostMetrics, Level, Thresholds};
use crate::util::{human_bytes, human_duration, pct};

pub const TILE_WIDTH: u16 = 30;
pub const TILE_HEIGHT: u16 = 7;

/// History shown alongside the current view.
#[derive(Debug, Clone, Default)]
pub struct HistoryView {
    pub start: Option<DateTime<Utc>>,
    /// CPU and memory percentages for the selected host, oldest first.
    pub cpu: Vec<Option<f64>>,
    pub mem: Vec<Option<f64>>,
}

pub struct ViewData<'a> {
    pub fleet: &'a FleetState,
    pub now: DateTime<Utc>,
    /// Set when showing a historical moment.
    pub at: Option<DateTime<Utc>>,
    pub thresholds: &'a Thresholds,
    pub interval_secs: f64,
    pub history: HistoryView,
    /// Extra context for historical views ("nearest record 01:05:30").
    pub note: Option<String>,
}

pub fn status_color(s: HostStatus) -> Color {
    match s {
        HostStatus::Ok => Color::Green,
        HostStatus::Deviation => Color::Cyan,
        HostStatus::Warning => Color::Yellow,
        HostStatus::Critical => Color::Red,
        HostStatus::Unreachable => Color::Magenta,
        HostStatus::Pending => Color::DarkGray,
    }
}

fn level_color(l: Level) -> Color {
    match l {
        Level::Ok => Color::Green,
        Level::Warning => Color::Yellow,
        Level::Critical => Color::Red,
    }
}

fn band_color(v: Option<f64>, band: Band) -> Color {
    v.map(|v| level_color(band.level(v)))
        .unwrap_or(Color::DarkGray)
}

/// A horizontal bar of `width` cells for a percentage.
pub fn bar(pct: Option<f64>, width: usize) -> String {
    let Some(p) = pct else {
        return "·".repeat(width);
    };
    let eighths = ((p.clamp(0.0, 100.0) / 100.0) * (width * 8) as f64).round() as usize;
    let full = eighths / 8;
    let rem = eighths % 8;
    let mut s = "█".repeat(full.min(width));
    if full < width {
        const PARTS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
        let mut used = full;
        if rem > 0 {
            s.push_str(PARTS[rem]);
            used += 1;
        }
        s.push_str(&"·".repeat(width - used));
    }
    s
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Greedy word wrap; words longer than `width` are split.
pub fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > width {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let head: String = word.chars().take(width).collect();
            word = word.chars().skip(width).collect();
            lines.push(head);
        }
        let needed = if cur.is_empty() {
            word.chars().count()
        } else {
            cur.chars().count() + 1 + word.chars().count()
        };
        if needed > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(&word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn rate(v: Option<f64>) -> String {
    v.map(|v| format!("{}/s", human_bytes(v)).replace(' ', ""))
        .unwrap_or_else(|| "n/a".into())
}

pub fn render(f: &mut Frame, app: &mut App, data: &ViewData) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .areas(f.area());
    render_header(f, header, app, data);
    match app.screen {
        Screen::Fleet => render_fleet(f, body, app, data),
        Screen::Host => render_host(f, body, app, data),
        Screen::Security => render_security(f, body, app, data),
    }
    render_footer(f, footer, app, data);
    if app.help {
        render_help(f, f.area());
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &App, data: &ViewData) {
    let mut spans = vec![
        Span::styled(
            " skry ",
            Style::new().bold().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(format!(" {} hosts ", data.fleet.hosts.len())),
    ];
    for (status, n) in data.fleet.counts() {
        if n > 0 {
            spans.push(Span::styled("● ", Style::new().fg(status_color(status))));
            spans.push(Span::raw(format!("{n} {}  ", status.label())));
        }
    }
    spans.push(Span::styled(
        format!("sort:{} ", app.sort.label()),
        Style::new().fg(Color::DarkGray),
    ));
    if app.status_filter != super::app::StatusFilter::All {
        spans.push(Span::styled(
            format!("show:{} ", app.status_filter.label()),
            Style::new().fg(Color::Yellow),
        ));
    }
    if !app.filter.is_empty() {
        spans.push(Span::styled(
            format!("filter:{} ", app.filter),
            Style::new().fg(Color::Yellow),
        ));
    }
    let left = Line::from(spans);
    let right = match data.at {
        None => Line::from(vec![
            Span::styled(
                " LIVE ",
                Style::new().bold().fg(Color::Black).bg(Color::Green),
            ),
            Span::raw(format!(" every {}s ", data.interval_secs)),
        ]),
        Some(t) => Line::from(vec![Span::styled(
            format!(
                " HISTORY {} (-{}) ",
                t.format("%Y-%m-%d %H:%M:%S"),
                human_duration((data.now - t).num_seconds() as f64)
            ),
            Style::new().bold().fg(Color::Black).bg(Color::Yellow),
        )]),
    };
    let [l, r] = Layout::horizontal([
        Constraint::Min(10),
        Constraint::Length(right.width() as u16),
    ])
    .areas(area);
    f.render_widget(Paragraph::new(left), l);
    f.render_widget(Paragraph::new(right), r);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App, data: &ViewData) {
    let [timeline, keys] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    // Timeline
    let line = match data.history.start {
        Some(start) if start < data.now => {
            let label_w = 22usize;
            let w = (timeline.width as usize)
                .saturating_sub(label_w + 2)
                .max(10);
            let total = (data.now - start).num_seconds().max(1) as f64;
            let pos = match data.at {
                Some(t) => {
                    ((t - start).num_seconds() as f64 / total * (w - 1) as f64).round() as usize
                }
                None => w - 1,
            }
            .min(w - 1);
            let mut track: Vec<Span> = Vec::new();
            track.push(Span::styled(
                format!(" {} ", start.format("%H:%M")),
                Style::new().fg(Color::DarkGray),
            ));
            track.push(Span::styled(
                "━".repeat(pos),
                Style::new().fg(Color::DarkGray),
            ));
            track.push(Span::styled(
                "●",
                Style::new().fg(if data.at.is_some() {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ));
            track.push(Span::styled(
                "━".repeat(w - 1 - pos),
                Style::new().fg(Color::DarkGray),
            ));
            track.push(Span::styled(" now", Style::new().fg(Color::DarkGray)));
            if let Some(n) = &data.note {
                track.push(Span::styled(
                    format!("  {n}"),
                    Style::new().fg(Color::Yellow),
                ));
            }
            Line::from(track)
        }
        _ => Line::from(Span::styled(
            " no history yet — timeline fills as samples are recorded",
            Style::new().fg(Color::DarkGray),
        )),
    };
    f.render_widget(Paragraph::new(line), timeline);

    let content = if app.editing_filter {
        Line::from(vec![
            Span::styled(" filter: ", Style::new().fg(Color::Yellow)),
            Span::raw(app.filter.clone()),
            Span::styled("▌", Style::new().fg(Color::Yellow)),
            Span::styled("  Enter apply  Esc clear", Style::new().fg(Color::DarkGray)),
        ])
    } else if let Some(m) = &app.message {
        Line::from(Span::styled(format!(" {m}"), Style::new().fg(Color::Cyan)))
    } else {
        let hints: &[(&str, &str)] = match app.screen {
            Screen::Fleet => &[
                ("←↑↓→", "move"),
                ("Enter", "details"),
                ("/", "filter"),
                ("f", "status"),
                ("o", "sort"),
                ("[ ]", "time"),
                ("S", "security"),
                ("s", "snapshot"),
                ("?", "help"),
                ("q", "quit"),
            ],
            Screen::Host => &[
                ("Esc", "back"),
                ("Tab", "panel"),
                ("n/p", "next/prev host"),
                ("↑↓", "scroll"),
                ("[ ]", "time"),
                ("s", "snapshot"),
                ("?", "help"),
                ("q", "quit"),
            ],
            Screen::Security => &[
                ("Esc", "back"),
                ("↑↓", "scroll"),
                ("[ ]", "time"),
                ("s", "snapshot"),
                ("q", "quit"),
            ],
        };
        let mut spans = vec![Span::raw(" ")];
        for (k, v) in hints {
            spans.push(Span::styled(*k, Style::new().bold().fg(Color::Cyan)));
            spans.push(Span::styled(
                format!(" {v}  "),
                Style::new().fg(Color::Gray),
            ));
        }
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(content), keys);
}

fn tile_lines(h: &HostState, t: &Thresholds, width: usize) -> Vec<Line<'static>> {
    let bar_w = width.saturating_sub(10).max(4);
    if h.conn == ConnState::Failed {
        let e = h.error.as_ref();
        let kind = e.map(|e| e.kind.label()).unwrap_or("unreachable");
        let msg = e.map(|e| e.message.clone()).unwrap_or_default();
        let mut lines = vec![Line::from(Span::styled(
            format!("✖ {kind}"),
            Style::new().bold().fg(Color::Magenta),
        ))];
        for chunk in wrap_words(&msg, width).into_iter().take(4) {
            lines.push(Line::from(Span::styled(
                chunk,
                Style::new().fg(Color::Gray),
            )));
        }
        return lines;
    }
    let Some(m) = &h.metrics else {
        let what = if h.conn == ConnState::Connecting {
            "connecting…"
        } else {
            "waiting…"
        };
        return vec![Line::from(Span::styled(
            what,
            Style::new().fg(Color::DarkGray),
        ))];
    };
    let row = |label: &'static str, v: Option<f64>, band: Band| {
        Line::from(vec![
            Span::styled(format!("{label} "), Style::new().fg(Color::Gray)),
            Span::styled(bar(v, bar_w), Style::new().fg(band_color(v, band))),
            Span::raw(format!(
                "{:>5}",
                v.map(|v| format!("{v:.0}%"))
                    .unwrap_or_else(|| "n/a".into())
            )),
        ])
    };
    let load = m
        .load
        .map(|l| format!("{:.2}", l.one))
        .unwrap_or_else(|| "n/a".into());
    let last = vec![
        Span::styled("LD ", Style::new().fg(Color::Gray)),
        Span::styled(
            load,
            Style::new().fg(band_color(m.load_per_core, t.load_per_core)),
        ),
        Span::styled(
            format!(" ↓{} ↑{}", rate(m.net_rx_bps), rate(m.net_tx_bps)),
            Style::new().fg(Color::Gray),
        ),
    ];
    let mut flags = Vec::new();
    if let Some(d) = h.deviations.first() {
        flags.push(Span::styled(
            format!("◆ {} z{:.1} ", d.metric.name(), d.z),
            Style::new().fg(Color::Cyan),
        ));
    }
    if let Probe::Ok(u) = &m.failed_units
        && !u.is_empty()
    {
        flags.push(Span::styled(
            format!("✖ {} unit ", u.len()),
            Style::new().fg(Color::Red),
        ));
    }
    if !h.findings.is_empty() {
        let worst = h.findings.iter().map(|f| f.level).max().unwrap_or_default();
        flags.push(Span::styled(
            format!("⚑ {} security", h.findings.len()),
            Style::new().fg(level_color(worst)),
        ));
    }
    vec![
        row("CPU", m.cpu_pct(), t.cpu),
        row("MEM", m.mem_pct(), t.memory),
        row("DSK", m.disk_max_pct(), t.disk),
        Line::from(last),
        Line::from(flags),
    ]
}

fn render_fleet(f: &mut Frame, area: Rect, app: &mut App, data: &ViewData) {
    let hosts = &data.fleet.hosts;
    let visible = app.visible(hosts);
    if visible.is_empty() {
        let msg = if hosts.is_empty() {
            "no hosts"
        } else {
            "no hosts match the filter (Esc to clear)"
        };
        f.render_widget(
            Paragraph::new(msg)
                .alignment(Alignment::Center)
                .fg(Color::DarkGray),
            area,
        );
        return;
    }
    let cols = (area.width / TILE_WIDTH).max(1) as usize;
    app.columns = cols;
    let tile_w = area.width / cols as u16;
    let rows_fit = (area.height / TILE_HEIGHT).max(1) as usize;
    let cursor = app.cursor(hosts, &visible);
    let cursor_row = cursor / cols;
    let first_row = cursor_row.saturating_sub(rows_fit - 1);
    for (n, &i) in visible.iter().enumerate().skip(first_row * cols) {
        let row = n / cols - first_row;
        if row >= rows_fit {
            break;
        }
        let col = n % cols;
        let rect = Rect {
            x: area.x + col as u16 * tile_w,
            y: area.y + row as u16 * TILE_HEIGHT,
            width: tile_w,
            height: TILE_HEIGHT,
        };
        let h = &hosts[i];
        let selected = n == cursor;
        let color = status_color(h.status);
        let mut block = Block::bordered()
            .border_type(if selected {
                BorderType::Thick
            } else {
                BorderType::Rounded
            })
            .border_style(Style::new().fg(color))
            .title(Line::from(Span::styled(
                format!(
                    " {} ",
                    truncate(&h.name, tile_w.saturating_sub(14) as usize)
                ),
                if selected {
                    Style::new().bold().fg(Color::Black).bg(color)
                } else {
                    Style::new().bold()
                },
            )))
            .title(
                Line::from(Span::styled(
                    format!(" {} ", h.status.label()),
                    Style::new().fg(color),
                ))
                .right_aligned(),
            );
        if let Some(g) = h.groups.first() {
            block = block.title_bottom(
                Line::from(Span::styled(
                    format!(" @{g} "),
                    Style::new().fg(Color::DarkGray),
                ))
                .right_aligned(),
            );
        }
        let inner_w = tile_w.saturating_sub(2) as usize;
        f.render_widget(
            Paragraph::new(tile_lines(h, data.thresholds, inner_w)).block(block),
            rect,
        );
    }
    let total_rows = visible.len().div_ceil(cols);
    if total_rows > rows_fit {
        let info = format!(
            " rows {}-{} of {} ",
            first_row + 1,
            (first_row + rows_fit).min(total_rows),
            total_rows
        );
        let w = info.len() as u16;
        let r = Rect {
            x: area.right().saturating_sub(w),
            y: area.bottom().saturating_sub(1),
            width: w.min(area.width),
            height: 1,
        };
        f.render_widget(Paragraph::new(info).fg(Color::DarkGray), r);
    }
}

fn selected_host<'a>(app: &App, data: &'a ViewData) -> Option<&'a HostState> {
    let hosts = &data.fleet.hosts;
    app.selected
        .as_ref()
        .and_then(|n| hosts.iter().find(|h| &h.name == n))
        .or_else(|| hosts.first())
}

fn kv(k: &str, v: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{k:<11}"), Style::new().fg(Color::Gray)),
        Span::raw(v.into()),
    ])
}

fn render_host(f: &mut Frame, area: Rect, app: &mut App, data: &ViewData) {
    let Some(h) = selected_host(app, data) else {
        return;
    };
    let color = status_color(h.status);
    let [title, tabs, body] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .areas(area);

    let m = h.metrics.as_ref();
    let mut top = vec![
        Span::styled(
            format!(" {} ", h.name),
            Style::new().bold().fg(Color::Black).bg(color),
        ),
        Span::styled(
            format!(" {} ", h.status.label()),
            Style::new().fg(color).bold(),
        ),
    ];
    if let Some(a) = &h.addr {
        top.push(Span::styled(format!(" {a}"), Style::new().fg(Color::Gray)));
    }
    let second = match m {
        Some(m) => format!(
            " {} · {} {} · up {} · {} cores{}",
            m.os.clone().unwrap_or_else(|| "unknown OS".into()),
            m.kernel,
            m.arch,
            m.uptime_secs
                .map(human_duration)
                .unwrap_or_else(|| "n/a".into()),
            m.cores,
            m.cpu_model
                .as_ref()
                .map(|c| format!(" · {c}"))
                .unwrap_or_default()
        ),
        None => String::new(),
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(top),
            Line::from(Span::styled(second, Style::new().fg(Color::Gray))),
        ]),
        title,
    );
    let titles: Vec<Line> = HostTab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| Line::from(format!("{} {}", i + 1, t.title())))
        .collect();
    let selected = HostTab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    f.render_widget(
        Tabs::new(titles)
            .select(selected)
            .highlight_style(
                Style::new()
                    .bold()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider("│"),
        tabs,
    );

    if let Some(e) = &h.error {
        let [err, rest] = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(body);
        f.render_widget(
            Paragraph::new(e.message.clone())
                .wrap(Wrap { trim: true })
                .fg(Color::Magenta)
                .block(Block::bordered().title(format!(
                    " {} since {} ",
                    e.kind.label(),
                    e.since.format("%H:%M:%S")
                ))),
            err,
        );
        if let Some(m) = m {
            host_tab(f, rest, app, data, h, m);
        }
        return;
    }
    match m {
        Some(m) => host_tab(f, body, app, data, h, m),
        None => f.render_widget(
            Paragraph::new("waiting for the first sample…").fg(Color::DarkGray),
            body,
        ),
    }
}

fn host_tab(f: &mut Frame, area: Rect, app: &App, data: &ViewData, h: &HostState, m: &HostMetrics) {
    match app.tab {
        HostTab::Overview => overview(f, area, data, h, m),
        HostTab::Processes => processes(f, area, app, m),
        HostTab::NetDisk => net_disk(f, area, m),
        HostTab::Services => services(f, area, m),
        HostTab::Security => host_security(f, area, h, m),
    }
}

fn gauge_line(
    label: &str,
    v: Option<f64>,
    band: Band,
    width: usize,
    extra: String,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<5}"), Style::new().fg(Color::Gray)),
        Span::styled(bar(v, width), Style::new().fg(band_color(v, band))),
        Span::raw(format!(" {:>6} ", pct(v))),
        Span::styled(extra, Style::new().fg(Color::Gray)),
    ])
}

fn overview(f: &mut Frame, area: Rect, data: &ViewData, h: &HostState, m: &HostMetrics) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    let t = data.thresholds;
    let bar_w = (left.width as usize).saturating_sub(34).clamp(8, 40);
    let mut lines = Vec::new();
    let cpu_extra = m
        .cpu
        .as_ref()
        .map(|c| {
            format!(
                "us {:.0} sy {:.0} io {:.0} st {:.0}",
                c.user_pct, c.system_pct, c.iowait_pct, c.steal_pct
            )
        })
        .unwrap_or_default();
    lines.push(gauge_line("CPU", m.cpu_pct(), t.cpu, bar_w, cpu_extra));
    let mem_extra = m
        .mem
        .as_ref()
        .map(|x| {
            format!(
                "{} / {}",
                human_bytes(x.used as f64),
                human_bytes(x.total as f64)
            )
        })
        .unwrap_or_default();
    lines.push(gauge_line("MEM", m.mem_pct(), t.memory, bar_w, mem_extra));
    if let Some(x) = &m.mem {
        lines.push(gauge_line(
            "SWAP",
            (x.swap_total > 0).then_some(x.swap_pct),
            Band::new(50.0, 80.0),
            bar_w,
            format!(
                "{} / {}",
                human_bytes(x.swap_used as f64),
                human_bytes(x.swap_total as f64)
            ),
        ));
    }
    lines.push(gauge_line(
        "DISK",
        m.disk_max_pct(),
        t.disk,
        bar_w,
        m.disks
            .iter()
            .max_by(|a, b| a.used_pct.total_cmp(&b.used_pct))
            .map(|d| d.mount.clone())
            .unwrap_or_default(),
    ));
    if let Some(l) = &m.load {
        lines.push(Line::from(vec![
            Span::styled("LOAD ", Style::new().fg(Color::Gray)),
            Span::styled(
                format!("{:.2} {:.2} {:.2}", l.one, l.five, l.fifteen),
                Style::new().fg(band_color(m.load_per_core, t.load_per_core)),
            ),
            Span::styled(
                format!(
                    "  {:.2}/core  {} tasks, {} running",
                    m.load_per_core.unwrap_or(0.0),
                    l.total,
                    l.running
                ),
                Style::new().fg(Color::Gray),
            ),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("NET  ", Style::new().fg(Color::Gray)),
        Span::raw(format!(
            "↓ {}  ↑ {}",
            rate(m.net_rx_bps),
            rate(m.net_tx_bps)
        )),
    ]));
    lines.push(Line::raw(""));
    if let Some(c) = &m.cpu {
        lines.push(Line::from(Span::styled("Per core", Style::new().bold())));
        let per_row = ((left.width as usize).saturating_sub(2) / 18).max(1);
        for chunk in c
            .cores
            .iter()
            .enumerate()
            .collect::<Vec<_>>()
            .chunks(per_row)
        {
            let mut spans = Vec::new();
            for (i, v) in chunk {
                spans.push(Span::styled(
                    format!("{i:>3} "),
                    Style::new().fg(Color::Gray),
                ));
                spans.push(Span::styled(
                    bar(Some(**v), 8),
                    Style::new().fg(band_color(Some(**v), t.cpu)),
                ));
                spans.push(Span::raw(format!("{:>4.0}% ", v)));
            }
            lines.push(Line::from(spans));
        }
    }
    let problems: Vec<Line> = h
        .health
        .breaches
        .iter()
        .map(|b| {
            Line::from(Span::styled(
                format!("▲ {}", b.describe()),
                Style::new().fg(level_color(b.level)),
            ))
        })
        .chain(h.deviations.iter().map(|d| {
            Line::from(Span::styled(
                format!("◆ {}", d.describe()),
                Style::new().fg(Color::Cyan),
            ))
        }))
        .chain(h.findings.iter().map(|x| {
            Line::from(Span::styled(
                format!("⚑ {}", x.message),
                Style::new().fg(level_color(x.level)),
            ))
        }))
        .collect();
    if !problems.is_empty() {
        lines.push(Line::raw(""));
        lines.extend(problems);
    }
    f.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::RIGHT)
                .border_style(Style::new().fg(Color::DarkGray)),
        ),
        left,
    );

    let [spark, info] = Layout::vertical([Constraint::Length(12), Constraint::Min(3)]).areas(right);
    let to_u64 = |v: &[Option<f64>]| -> Vec<u64> {
        v.iter().map(|x| x.unwrap_or(0.0).round() as u64).collect()
    };
    let [s1, s2] = Layout::vertical([Constraint::Length(6), Constraint::Length(6)]).areas(spark);
    let cpu = to_u64(&data.history.cpu);
    let mem = to_u64(&data.history.mem);
    let sw = s1.width.saturating_sub(2) as usize;
    let tail = |v: &[u64]| v[v.len().saturating_sub(sw)..].to_vec();
    f.render_widget(
        Sparkline::default()
            .block(
                Block::new()
                    .title(" CPU history ")
                    .borders(Borders::TOP)
                    .border_style(Style::new().fg(Color::DarkGray)),
            )
            .data(tail(&cpu))
            .max(100)
            .style(Style::new().fg(Color::Green)),
        s1,
    );
    f.render_widget(
        Sparkline::default()
            .block(
                Block::new()
                    .title(" Memory history ")
                    .borders(Borders::TOP)
                    .border_style(Style::new().fg(Color::DarkGray)),
            )
            .data(tail(&mem))
            .max(100)
            .style(Style::new().fg(Color::Blue)),
        s2,
    );
    let mut info_lines = vec![kv("Hostname", m.hostname.clone())];
    let ips: Vec<String> = m
        .ips
        .iter()
        .filter(|i| i.scope != "host")
        .map(|i| format!("{} {}", i.iface, i.cidr))
        .collect();
    for (n, ip) in ips.iter().take(4).enumerate() {
        info_lines.push(kv(if n == 0 { "Address" } else { "" }, ip.clone()));
    }
    info_lines.push(kv("Procs", m.proc_count.to_string()));
    info_lines.push(kv(
        "Units",
        match &m.failed_units {
            Probe::Ok(u) if u.is_empty() => "no failed units".to_string(),
            Probe::Ok(u) => format!("{} failed", u.len()),
            p => p.describe().unwrap_or_default(),
        },
    ));
    info_lines.push(kv(
        "Containers",
        match &m.containers {
            Probe::Ok(c) => format!(
                "{} running / {}",
                c.iter().filter(|c| c.state == "running").count(),
                c.len()
            ),
            p => p.describe().unwrap_or_default(),
        },
    ));
    info_lines.push(kv("Sampled", m.ts.format("%H:%M:%S UTC").to_string()));
    f.render_widget(
        Paragraph::new(info_lines).block(Block::new().padding(ratatui::widgets::Padding::left(1))),
        info,
    );
}

fn header_row(cells: &[&'static str]) -> Row<'static> {
    Row::new(cells.iter().map(|c| Cell::from(*c))).style(Style::new().bold().fg(Color::Cyan))
}

fn processes(f: &mut Frame, area: Rect, app: &App, m: &HostMetrics) {
    let rows: Vec<Row> = m
        .procs
        .iter()
        .skip(app.scroll)
        .map(|p| {
            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(truncate(&p.user, 10)),
                Cell::from(
                    p.cpu_pct
                        .map(|c| format!("{c:.1}"))
                        .unwrap_or_else(|| "…".into()),
                ),
                Cell::from(format!("{:.1}", p.mem_pct)),
                Cell::from(human_bytes(p.rss as f64)),
                Cell::from(p.state.clone()),
                Cell::from(p.name.clone()),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(11),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(3),
            Constraint::Min(10),
        ],
    )
    .header(header_row(&[
        "PID", "USER", "CPU%", "MEM%", "RSS", "S", "COMMAND",
    ]))
    .block(Block::new().title(format!(
        " top processes by CPU and memory · {} total ",
        m.proc_count
    )));
    f.render_widget(table, area);
}

fn net_disk(f: &mut Frame, area: Rect, m: &HostMetrics) {
    let [top, mid, bottom] = Layout::vertical([
        Constraint::Length(m.net.len().clamp(1, 8) as u16 + 2),
        Constraint::Length(m.disk_io.len().clamp(1, 8) as u16 + 2),
        Constraint::Min(3),
    ])
    .areas(area);
    let net = Table::new(
        m.net.iter().map(|n| {
            Row::new(vec![
                n.iface.clone(),
                rate(Some(n.rx_bps)),
                rate(Some(n.tx_bps)),
                human_bytes(n.rx_total as f64),
                human_bytes(n.tx_total as f64),
                format!("{}/{}", n.errors, n.drops),
            ])
        }),
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(11),
            Constraint::Length(11),
            Constraint::Min(8),
        ],
    )
    .header(header_row(&[
        "IFACE", "RX", "TX", "RX TOTAL", "TX TOTAL", "ERR/DROP",
    ]))
    .block(Block::new().title(" network "));
    f.render_widget(net, top);
    let io = Table::new(
        m.disk_io.iter().map(|d| {
            Row::new(vec![
                d.device.clone(),
                rate(Some(d.read_bps)),
                rate(Some(d.write_bps)),
                format!("{:.0}", d.read_iops),
                format!("{:.0}", d.write_iops),
                format!("{:.0}%", d.util_pct),
            ])
        }),
        [
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(6),
        ],
    )
    .header(header_row(&[
        "DEVICE", "READ", "WRITE", "R IOPS", "W IOPS", "UTIL",
    ]))
    .block(Block::new().title(" disk I/O "));
    f.render_widget(io, mid);
    let usage = Table::new(
        m.disks.iter().map(|d| {
            Row::new(vec![
                Cell::from(truncate(&d.mount, 24)),
                Cell::from(truncate(&d.filesystem, 20)),
                Cell::from(human_bytes(d.used as f64)),
                Cell::from(human_bytes(d.total as f64)),
                Cell::from(Line::from(vec![
                    Span::styled(
                        bar(Some(d.used_pct), 12),
                        Style::new().fg(band_color(Some(d.used_pct), Band::new(80.0, 90.0))),
                    ),
                    Span::raw(format!(" {:.1}%", d.used_pct)),
                ])),
            ])
        }),
        [
            Constraint::Length(25),
            Constraint::Length(21),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(20),
        ],
    )
    .header(header_row(&["MOUNT", "FILESYSTEM", "USED", "SIZE", "USE"]))
    .block(Block::new().title(" filesystems "));
    f.render_widget(usage, bottom);
}

fn services(f: &mut Frame, area: Rect, m: &HostMetrics) {
    let [units, containers] =
        Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(area);
    match &m.failed_units {
        Probe::Ok(u) if !u.is_empty() => {
            let t = Table::new(
                u.iter().map(|x| {
                    Row::new(vec![
                        x.unit.clone(),
                        format!("{}/{}", x.active, x.sub),
                        x.description.clone(),
                    ])
                    .style(Style::new().fg(Color::Red))
                }),
                [
                    Constraint::Length(36),
                    Constraint::Length(16),
                    Constraint::Min(10),
                ],
            )
            .header(header_row(&["FAILED UNIT", "STATE", "DESCRIPTION"]))
            .block(Block::new().title(" systemd "));
            f.render_widget(t, units);
        }
        p => {
            let text = match p {
                Probe::Ok(_) => {
                    Span::styled("no failed systemd units", Style::new().fg(Color::Green))
                }
                other => Span::styled(
                    format!("failed units: {}", other.describe().unwrap_or_default()),
                    Style::new().fg(Color::DarkGray),
                ),
            };
            f.render_widget(
                Paragraph::new(Line::from(text)).block(Block::new().title(" systemd ")),
                units,
            );
        }
    }
    match &m.containers {
        Probe::Ok(c) if !c.is_empty() => {
            let t = Table::new(
                c.iter().map(|x| {
                    let style = if x.state == "running" {
                        Style::new()
                    } else {
                        Style::new().fg(Color::DarkGray)
                    };
                    Row::new(vec![
                        truncate(&x.name, 28),
                        truncate(&x.image, 28),
                        truncate(&x.status, 24),
                        pct(x.cpu_pct),
                        x.mem_usage.clone().unwrap_or_else(|| "n/a".into()),
                    ])
                    .style(style)
                }),
                [
                    Constraint::Length(29),
                    Constraint::Length(29),
                    Constraint::Length(25),
                    Constraint::Length(8),
                    Constraint::Min(10),
                ],
            )
            .header(header_row(&[
                "CONTAINER",
                "IMAGE",
                "STATUS",
                "CPU",
                "MEMORY",
            ]))
            .block(Block::new().title(format!(
                " containers ({}) ",
                c.first().map(|x| x.runtime.as_str()).unwrap_or("")
            )));
            f.render_widget(t, containers);
        }
        p => {
            let text = match p {
                Probe::Ok(_) => "no containers".to_string(),
                other => format!("containers: {}", other.describe().unwrap_or_default()),
            };
            f.render_widget(
                Paragraph::new(text)
                    .fg(Color::DarkGray)
                    .block(Block::new().title(" containers ")),
                containers,
            );
        }
    }
}

fn host_security(f: &mut Frame, area: Rect, h: &HostState, m: &HostMetrics) {
    let mut lines = Vec::new();
    let allowed: Option<std::collections::BTreeSet<u16>> = h
        .allowed_ports
        .as_ref()
        .map(|p| p.iter().copied().collect());
    lines.push(Line::from(Span::styled(
        "Listening TCP ports",
        Style::new().bold(),
    )));
    match &m.ports {
        Probe::Ok(ports) => {
            for p in ports {
                let exposed = !(p.addr.starts_with("127.") || p.addr == "::1");
                let bad = exposed && allowed.as_ref().is_some_and(|a| !a.contains(&p.port));
                let style = if bad {
                    Style::new().fg(Color::Red).bold()
                } else if exposed {
                    Style::new()
                } else {
                    Style::new().fg(Color::DarkGray)
                };
                let note = if bad {
                    "  not in allowlist"
                } else if !exposed {
                    "  loopback"
                } else {
                    ""
                };
                lines.push(Line::from(Span::styled(
                    format!("  {:>5}  {}{note}", p.port, p.addr),
                    style,
                )));
            }
        }
        p => lines.push(Line::from(format!(
            "  {}",
            p.describe().unwrap_or_default()
        ))),
    }
    if let Some(a) = &allowed {
        lines.push(Line::from(Span::styled(
            format!(
                "  allowlist: {}",
                a.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
            ),
            Style::new().fg(Color::Gray),
        )));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Failed SSH logins (24h)",
        Style::new().bold(),
    )));
    match &m.failed_logins {
        Probe::Ok(l) => {
            lines.push(Line::from(format!(
                "  {} events from {}",
                l.total, l.source
            )));
            for (ip, n) in l.top_sources.iter().take(8) {
                lines.push(Line::from(format!("  {n:>6}  {ip}")));
            }
        }
        p => lines.push(Line::from(format!(
            "  {}",
            p.describe().unwrap_or_default()
        ))),
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Pending updates",
        Style::new().bold(),
    )));
    match &m.updates {
        Probe::Ok(u) => {
            let sec = u
                .security
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into());
            let style = if u.security.unwrap_or(0) > 0 {
                Style::new().fg(Color::Yellow)
            } else {
                Style::new()
            };
            lines.push(Line::from(Span::styled(
                format!("  {} total, {} security ({})", u.total, sec, u.manager),
                style,
            )));
            if !u.security_packages.is_empty() {
                lines.push(Line::from(format!(
                    "  {}",
                    truncate(
                        &u.security_packages.join(", "),
                        area.width.saturating_sub(4) as usize
                    )
                )));
            }
        }
        p => lines.push(Line::from(format!(
            "  {}",
            p.describe().unwrap_or_default()
        ))),
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn render_security(f: &mut Frame, area: Rect, app: &App, data: &ViewData) {
    let tls_h = if data.fleet.tls.is_empty() {
        0
    } else {
        data.fleet.tls.len().min(8) as u16 + 3
    };
    let [hosts_area, tls_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(tls_h)]).areas(area);
    let rows: Vec<Row> = data
        .fleet
        .hosts
        .iter()
        .skip(app.scroll)
        .map(|h| {
            let Some(m) = &h.metrics else {
                return Row::new(vec![
                    Cell::from(h.name.clone()),
                    Cell::from(
                        h.error
                            .as_ref()
                            .map(|e| e.kind.label())
                            .unwrap_or("pending"),
                    ),
                ])
                .style(Style::new().fg(Color::DarkGray));
            };
            let logins = match &m.failed_logins {
                Probe::Ok(l) => l.total.to_string(),
                _ => "n/a".into(),
            };
            let updates = match &m.updates {
                Probe::Ok(u) => format!(
                    "{} / {}",
                    u.security
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "?".into()),
                    u.total
                ),
                _ => "n/a".into(),
            };
            let allowed: Option<std::collections::BTreeSet<u16>> = h
                .allowed_ports
                .as_ref()
                .map(|p| p.iter().copied().collect());
            let ports = match (&m.ports, &allowed) {
                (Probe::Ok(p), Some(a)) => {
                    let bad = crate::security::unexpected_ports(p, a);
                    if bad.is_empty() {
                        "ok".into()
                    } else {
                        bad.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
                    }
                }
                (Probe::Ok(p), None) => {
                    let mut set: Vec<u16> = p.iter().map(|x| x.port).collect();
                    set.dedup();
                    format!(
                        "{} (no policy)",
                        set.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
                    )
                }
                _ => "n/a".into(),
            };
            let level = h.findings.iter().map(|x| x.level).max().unwrap_or_default();
            Row::new(vec![
                Cell::from(h.name.clone()),
                Cell::from(Span::styled(
                    level.as_str(),
                    Style::new().fg(level_color(level)),
                )),
                Cell::from(logins),
                Cell::from(updates),
                Cell::from(ports),
                Cell::from(
                    h.findings
                        .iter()
                        .map(|x| x.message.clone())
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            ])
        })
        .collect();
    let t = Table::new(
        rows,
        [
            Constraint::Length(22),
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Min(10),
        ],
    )
    .header(header_row(&[
        "HOST",
        "PULSE",
        "LOGINS",
        "SEC/UPD",
        "UNEXPECTED PORTS",
        "FINDINGS",
    ]))
    .block(Block::bordered().title(
        " security pulse · failed SSH logins (24h), pending security updates, listening ports ",
    ));
    f.render_widget(t, hosts_area);
    if tls_h > 0 {
        let t = Table::new(
            data.fleet.tls.iter().map(|r| {
                Row::new(vec![
                    Cell::from(r.endpoint.clone()),
                    Cell::from(Span::styled(
                        r.level.as_str(),
                        Style::new().fg(level_color(r.level)),
                    )),
                    Cell::from(r.describe()),
                    Cell::from(r.subject.clone().unwrap_or_default()),
                    Cell::from(r.issuer.clone().unwrap_or_default()),
                ])
            }),
            [
                Constraint::Length(30),
                Constraint::Length(9),
                Constraint::Length(28),
                Constraint::Length(24),
                Constraint::Min(10),
            ],
        )
        .header(header_row(&[
            "ENDPOINT", "STATUS", "EXPIRY", "SUBJECT", "ISSUER",
        ]))
        .block(Block::bordered().title(" TLS certificates (checked from this machine) "));
        f.render_widget(t, tls_area);
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let lines: Vec<Line> = [
        ("Fleet", ""),
        ("  ←↑↓→ / hjkl", "move between tiles"),
        ("  Enter", "open host details"),
        ("  /", "filter by name or group"),
        ("  f", "cycle status filter (all, problems, unreachable)"),
        ("  o", "cycle sort (status, name, cpu, memory, disk)"),
        ("  S", "security pulse"),
        ("Host details", ""),
        ("  Tab / Shift-Tab, 1-5", "switch panel"),
        ("  n / p", "next / previous host"),
        ("  ↑↓ PgUp PgDn", "scroll"),
        ("  Esc", "back to the fleet"),
        ("Timeline", ""),
        ("  [ / ]", "one minute back / forward"),
        ("  { / }", "15 minutes back / forward"),
        ("  L / End", "return to live"),
        ("Anywhere", ""),
        ("  s", "incident snapshot (Markdown + JSON)"),
        ("  ?", "this help"),
        ("  q / Ctrl-C", "quit"),
    ]
    .iter()
    .map(|(k, v)| {
        if v.is_empty() {
            Line::from(Span::styled(*k, Style::new().bold().fg(Color::Cyan)))
        } else {
            Line::from(vec![
                Span::styled(format!("{k:<24}"), Style::new().bold()),
                Span::raw(*v),
            ])
        }
    })
    .collect();
    let w = 76.min(area.width);
    let h = (lines.len() as u16 + 2).min(area.height);
    let rect = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title(" keys · any key to close ")
                .border_style(Style::new().fg(Color::Cyan)),
        ),
        rect,
    );
}
