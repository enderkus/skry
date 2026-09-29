//! TUI state and key handling. Pure: no terminal or I/O here.

use chrono::{DateTime, Duration, Utc};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::engine::{HostState, HostStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Fleet,
    Host,
    Security,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTab {
    Overview,
    Processes,
    NetDisk,
    Services,
    Security,
}

impl HostTab {
    pub const ALL: [HostTab; 5] = [
        HostTab::Overview,
        HostTab::Processes,
        HostTab::NetDisk,
        HostTab::Services,
        HostTab::Security,
    ];

    pub fn title(self) -> &'static str {
        match self {
            HostTab::Overview => "Overview",
            HostTab::Processes => "Processes",
            HostTab::NetDisk => "Network & Disks",
            HostTab::Services => "Services & Containers",
            HostTab::Security => "Security",
        }
    }

    fn index(self) -> usize {
        HostTab::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    fn cycle(self, delta: isize) -> HostTab {
        let n = HostTab::ALL.len() as isize;
        HostTab::ALL[((self.index() as isize + delta).rem_euclid(n)) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Status,
    Name,
    Cpu,
    Memory,
    Disk,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Status => "status",
            SortKey::Name => "name",
            SortKey::Cpu => "cpu",
            SortKey::Memory => "memory",
            SortKey::Disk => "disk",
        }
    }

    fn next(self) -> SortKey {
        match self {
            SortKey::Status => SortKey::Name,
            SortKey::Name => SortKey::Cpu,
            SortKey::Cpu => SortKey::Memory,
            SortKey::Memory => SortKey::Disk,
            SortKey::Disk => SortKey::Status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Problems,
    Unreachable,
}

impl StatusFilter {
    pub fn label(self) -> &'static str {
        match self {
            StatusFilter::All => "all",
            StatusFilter::Problems => "problems",
            StatusFilter::Unreachable => "unreachable",
        }
    }

    fn next(self) -> StatusFilter {
        match self {
            StatusFilter::All => StatusFilter::Problems,
            StatusFilter::Problems => StatusFilter::Unreachable,
            StatusFilter::Unreachable => StatusFilter::All,
        }
    }

    pub fn accepts(self, s: HostStatus) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Problems => !matches!(s, HostStatus::Ok | HostStatus::Pending),
            StatusFilter::Unreachable => s == HostStatus::Unreachable,
        }
    }
}

/// Where the timeline cursor is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeCursor {
    Live,
    At(DateTime<Utc>),
}

/// Something the event loop must do in response to a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    Snapshot,
    /// The time cursor moved; historical data must be (re)loaded.
    TimeChanged,
}

#[derive(Debug)]
pub struct App {
    pub screen: Screen,
    pub tab: HostTab,
    /// Selected host by name, so the selection survives re-sorting.
    pub selected: Option<String>,
    pub sort: SortKey,
    pub filter: String,
    pub editing_filter: bool,
    pub status_filter: StatusFilter,
    pub help: bool,
    pub time: TimeCursor,
    pub message: Option<String>,
    pub scroll: usize,
    /// Tiles per row in the last render, for up/down navigation.
    pub columns: usize,
    /// Oldest point of available history.
    pub history_start: Option<DateTime<Utc>>,
}

impl Default for App {
    fn default() -> Self {
        App {
            screen: Screen::Fleet,
            tab: HostTab::Overview,
            selected: None,
            sort: SortKey::Status,
            filter: String::new(),
            editing_filter: false,
            status_filter: StatusFilter::All,
            help: false,
            time: TimeCursor::Live,
            message: None,
            scroll: 0,
            columns: 1,
            history_start: None,
        }
    }
}

fn status_order(s: HostStatus) -> u8 {
    match s {
        HostStatus::Unreachable => 0,
        HostStatus::Critical => 1,
        HostStatus::Warning => 2,
        HostStatus::Deviation => 3,
        HostStatus::Ok => 4,
        HostStatus::Pending => 5,
    }
}

fn metric(h: &HostState, key: SortKey) -> f64 {
    let m = h.metrics.as_ref();
    match key {
        SortKey::Cpu => m.and_then(|m| m.cpu_pct()),
        SortKey::Memory => m.and_then(|m| m.mem_pct()),
        SortKey::Disk => m.and_then(|m| m.disk_max_pct()),
        _ => None,
    }
    .unwrap_or(-1.0)
}

impl App {
    /// Indices of hosts to show, filtered and sorted.
    pub fn visible(&self, hosts: &[HostState]) -> Vec<usize> {
        let needle = self.filter.to_lowercase();
        let mut idx: Vec<usize> = hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| self.status_filter.accepts(h.status))
            .filter(|(_, h)| {
                needle.is_empty()
                    || h.name.to_lowercase().contains(&needle)
                    || h.groups.iter().any(|g| g.to_lowercase().contains(&needle))
                    || h.metrics
                        .as_ref()
                        .is_some_and(|m| m.hostname.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect();
        idx.sort_by(|&a, &b| {
            let (ha, hb) = (&hosts[a], &hosts[b]);
            let by_name = ha.name.cmp(&hb.name);
            match self.sort {
                SortKey::Name => by_name,
                SortKey::Status => status_order(ha.status)
                    .cmp(&status_order(hb.status))
                    .then(by_name),
                key => metric(hb, key).total_cmp(&metric(ha, key)).then(by_name),
            }
        });
        idx
    }

    /// Position of the selection within `visible`, defaulting to the first.
    pub fn cursor(&self, hosts: &[HostState], visible: &[usize]) -> usize {
        self.selected
            .as_ref()
            .and_then(|name| visible.iter().position(|&i| &hosts[i].name == name))
            .unwrap_or(0)
    }

    fn select_at(&mut self, hosts: &[HostState], visible: &[usize], pos: usize) {
        if let Some(&i) = visible.get(pos) {
            self.selected = Some(hosts[i].name.clone());
        }
    }

    fn move_cursor(&mut self, hosts: &[HostState], delta: isize) {
        let visible = self.visible(hosts);
        if visible.is_empty() {
            return;
        }
        let cur = self.cursor(hosts, &visible) as isize;
        let pos = (cur + delta).clamp(0, visible.len() as isize - 1) as usize;
        self.select_at(hosts, &visible, pos);
    }

    fn step_time(&mut self, delta: Duration, now: DateTime<Utc>) -> Action {
        let current = match self.time {
            TimeCursor::Live => now,
            TimeCursor::At(t) => t,
        };
        let mut t = current + delta;
        if let Some(start) = self.history_start {
            t = t.max(start);
        } else if delta < Duration::zero() {
            self.message = Some("no history recorded yet".into());
            return Action::None;
        }
        self.time = if t >= now {
            TimeCursor::Live
        } else {
            TimeCursor::At(t)
        };
        Action::TimeChanged
    }

    pub fn handle_key(&mut self, key: KeyEvent, hosts: &[HostState], now: DateTime<Utc>) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Action::Quit;
        }
        if self.editing_filter {
            match key.code {
                KeyCode::Enter => self.editing_filter = false,
                KeyCode::Esc => {
                    self.editing_filter = false;
                    self.filter.clear();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            }
            let visible = self.visible(hosts);
            if !visible.is_empty()
                && self
                    .selected
                    .as_ref()
                    .is_none_or(|s| !visible.iter().any(|&i| &hosts[i].name == s))
            {
                self.select_at(hosts, &visible, 0);
            }
            return Action::None;
        }
        if self.help {
            self.help = false;
            return Action::None;
        }
        self.message = None;
        match key.code {
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Char('?') | KeyCode::F(1) => self.help = true,
            KeyCode::Char('s') => return Action::Snapshot,
            KeyCode::Char('[') => return self.step_time(-Duration::minutes(1), now),
            KeyCode::Char(']') => return self.step_time(Duration::minutes(1), now),
            KeyCode::Char('{') => return self.step_time(-Duration::minutes(15), now),
            KeyCode::Char('}') => return self.step_time(Duration::minutes(15), now),
            KeyCode::Char('L') | KeyCode::End if self.time != TimeCursor::Live => {
                self.time = TimeCursor::Live;
                return Action::TimeChanged;
            }
            KeyCode::Char('S') => {
                self.screen = if self.screen == Screen::Security {
                    Screen::Fleet
                } else {
                    Screen::Security
                };
                self.scroll = 0;
            }
            _ => match self.screen {
                Screen::Fleet => self.fleet_key(key, hosts),
                Screen::Host => self.host_key(key, hosts),
                Screen::Security => self.security_key(key),
            },
        }
        Action::None
    }

    fn fleet_key(&mut self, key: KeyEvent, hosts: &[HostState]) {
        let cols = self.columns.max(1) as isize;
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(hosts, -1),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(hosts, 1),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(hosts, -cols),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(hosts, cols),
            KeyCode::Home | KeyCode::Char('g') => self.move_cursor(hosts, isize::MIN / 2),
            KeyCode::Char('G') => self.move_cursor(hosts, isize::MAX / 2),
            KeyCode::Enter => {
                let visible = self.visible(hosts);
                if !visible.is_empty() {
                    let pos = self.cursor(hosts, &visible);
                    self.select_at(hosts, &visible, pos);
                    self.screen = Screen::Host;
                    self.scroll = 0;
                }
            }
            KeyCode::Char('/') => self.editing_filter = true,
            KeyCode::Char('f') => self.status_filter = self.status_filter.next(),
            KeyCode::Char('o') => self.sort = self.sort.next(),
            KeyCode::Esc => {
                self.filter.clear();
                self.status_filter = StatusFilter::All;
            }
            _ => {}
        }
    }

    fn host_key(&mut self, key: KeyEvent, hosts: &[HostState]) {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => {
                self.screen = Screen::Fleet;
                self.scroll = 0;
            }
            KeyCode::Tab | KeyCode::Right => {
                self.tab = self.tab.cycle(1);
                self.scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = self.tab.cycle(-1);
                self.scroll = 0;
            }
            KeyCode::Char(c @ '1'..='5') => {
                self.tab = HostTab::ALL[(c as u8 - b'1') as usize];
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Char('n') => self.move_cursor(hosts, 1),
            KeyCode::Char('p') => self.move_cursor(hosts, -1),
            _ => {}
        }
    }

    fn security_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Fleet,
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Target;
    use crate::model::{CpuUsage, HostMetrics};

    fn host(name: &str, status: HostStatus, cpu: f64) -> HostState {
        let t = Target {
            spec: name.into(),
            groups: vec!["g".into()],
            allowed_ports: None,
        };
        let mut h = HostState::new(&t, None);
        h.status = status;
        h.metrics = Some(HostMetrics {
            cpu: Some(CpuUsage {
                total_pct: cpu,
                ..Default::default()
            }),
            ..Default::default()
        });
        h
    }

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn hosts() -> Vec<HostState> {
        vec![
            host("web1", HostStatus::Ok, 10.0),
            host("db1", HostStatus::Critical, 90.0),
            host("web2", HostStatus::Unreachable, 0.0),
            host("cache", HostStatus::Warning, 50.0),
        ]
    }

    fn names(app: &App, hosts: &[HostState]) -> Vec<String> {
        app.visible(hosts)
            .iter()
            .map(|&i| hosts[i].name.clone())
            .collect()
    }

    #[test]
    fn sorting() {
        let h = hosts();
        let mut app = App::default();
        assert_eq!(names(&app, &h), ["web2", "db1", "cache", "web1"]);
        let now = Utc::now();
        app.handle_key(key(KeyCode::Char('o')), &h, now);
        assert_eq!(names(&app, &h), ["cache", "db1", "web1", "web2"]);
        app.handle_key(key(KeyCode::Char('o')), &h, now);
        assert_eq!(names(&app, &h), ["db1", "cache", "web1", "web2"]);
    }

    #[test]
    fn filtering() {
        let h = hosts();
        let mut app = App::default();
        let now = Utc::now();
        for k in [
            KeyCode::Char('/'),
            KeyCode::Char('w'),
            KeyCode::Char('e'),
            KeyCode::Enter,
        ] {
            app.handle_key(key(k), &h, now);
        }
        assert_eq!(names(&app, &h), ["web2", "web1"]);
        assert_eq!(app.selected.as_deref(), Some("web2"));
        app.handle_key(key(KeyCode::Char('f')), &h, now);
        assert_eq!(names(&app, &h), ["web2"]);
        app.handle_key(key(KeyCode::Esc), &h, now);
        assert_eq!(names(&app, &h).len(), 4);
    }

    #[test]
    fn navigation_and_enter() {
        let h = hosts();
        let mut app = App {
            columns: 2,
            ..Default::default()
        };
        let now = Utc::now();
        app.handle_key(key(KeyCode::Down), &h, now);
        assert_eq!(app.selected.as_deref(), Some("cache"));
        app.handle_key(key(KeyCode::Right), &h, now);
        assert_eq!(app.selected.as_deref(), Some("web1"));
        app.handle_key(key(KeyCode::Right), &h, now);
        assert_eq!(app.selected.as_deref(), Some("web1"), "clamped at the end");
        app.handle_key(key(KeyCode::Enter), &h, now);
        assert_eq!(app.screen, Screen::Host);
        app.handle_key(key(KeyCode::Tab), &h, now);
        assert_eq!(app.tab, HostTab::Processes);
        app.handle_key(key(KeyCode::Char('5')), &h, now);
        assert_eq!(app.tab, HostTab::Security);
        app.handle_key(key(KeyCode::Esc), &h, now);
        assert_eq!(app.screen, Screen::Fleet);
    }

    #[test]
    fn help_and_quit() {
        let h = hosts();
        let mut app = App::default();
        let now = Utc::now();
        app.handle_key(key(KeyCode::Char('?')), &h, now);
        assert!(app.help);
        assert_eq!(
            app.handle_key(key(KeyCode::Char('q')), &h, now),
            Action::None
        );
        assert!(!app.help);
        assert_eq!(
            app.handle_key(key(KeyCode::Char('q')), &h, now),
            Action::Quit
        );
        assert_eq!(
            app.handle_key(key(KeyCode::Char('s')), &h, now),
            Action::Snapshot
        );
    }

    #[test]
    fn timeline_scrubbing() {
        let h = hosts();
        let now = Utc::now();
        let mut app = App::default();
        assert_eq!(
            app.handle_key(key(KeyCode::Char('[')), &h, now),
            Action::None
        );
        assert!(app.message.is_some(), "no history yet");
        app.history_start = Some(now - Duration::minutes(30));
        assert_eq!(
            app.handle_key(key(KeyCode::Char('[')), &h, now),
            Action::TimeChanged
        );
        assert_eq!(app.time, TimeCursor::At(now - Duration::minutes(1)));
        app.handle_key(key(KeyCode::Char('{')), &h, now);
        app.handle_key(key(KeyCode::Char('{')), &h, now);
        assert_eq!(
            app.time,
            TimeCursor::At(now - Duration::minutes(30)),
            "clamped to history"
        );
        app.handle_key(key(KeyCode::Char('}')), &h, now);
        assert_eq!(app.time, TimeCursor::At(now - Duration::minutes(15)));
        app.handle_key(key(KeyCode::Char('}')), &h, now);
        assert_eq!(app.time, TimeCursor::Live);
        app.handle_key(key(KeyCode::Char('[')), &h, now);
        assert_eq!(
            app.handle_key(key(KeyCode::Char('L')), &h, now),
            Action::TimeChanged
        );
        assert_eq!(app.time, TimeCursor::Live);
    }

    #[test]
    fn security_screen_toggle() {
        let h = hosts();
        let mut app = App::default();
        let now = Utc::now();
        app.handle_key(key(KeyCode::Char('S')), &h, now);
        assert_eq!(app.screen, Screen::Security);
        app.handle_key(key(KeyCode::Char('S')), &h, now);
        assert_eq!(app.screen, Screen::Fleet);
    }
}
