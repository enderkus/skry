//! Rendering snapshot tests using ratatui's `TestBackend` and insta.

use chrono::{DateTime, Duration, TimeZone, Utc};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, HostTab, Screen};
use super::demo;
use super::ui::{self, HistoryView, ViewData};
use crate::engine::FleetState;
use crate::model::Thresholds;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 29, 12, 0, 0).unwrap()
}

fn draw(
    app: &mut App,
    fleet: &FleetState,
    w: u16,
    h: u16,
    at: Option<DateTime<Utc>>,
) -> Terminal<TestBackend> {
    let thresholds = Thresholds::default();
    let history = HistoryView {
        start: Some(now() - Duration::hours(3)),
        cpu: (0..60)
            .map(|i| Some(20.0 + (i % 12) as f64 * 5.0))
            .collect(),
        mem: (0..60).map(|i| Some(40.0 + (i % 5) as f64)).collect(),
    };
    let data = ViewData {
        fleet,
        now: now(),
        at,
        thresholds: &thresholds,
        interval_secs: 2.0,
        history,
        note: None,
    };
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| ui::render(f, app, &data)).unwrap();
    terminal
}

fn press(app: &mut App, fleet: &FleetState, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE), &fleet.hosts, now());
}

#[test]
fn fleet_grid() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App::default();
    let t = draw(&mut app, &fleet, 120, 32, None);
    assert_eq!(app.columns, 4);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn fleet_grid_narrow_terminal() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App::default();
    let t = draw(&mut app, &fleet, 64, 20, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn fleet_filtered_and_sorted() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App::default();
    for c in ['/', 'w', 'e', 'b'] {
        press(&mut app, &fleet, KeyCode::Char(c));
    }
    press(&mut app, &fleet, KeyCode::Enter);
    press(&mut app, &fleet, KeyCode::Char('o'));
    let t = draw(&mut app, &fleet, 100, 16, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn host_overview() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App {
        screen: Screen::Host,
        selected: Some("db-1".into()),
        ..Default::default()
    };
    let t = draw(&mut app, &fleet, 120, 36, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn host_tabs() {
    let fleet = demo::fleet(now(), 0.0);
    for tab in [
        HostTab::Processes,
        HostTab::NetDisk,
        HostTab::Services,
        HostTab::Security,
    ] {
        let mut app = App {
            screen: Screen::Host,
            selected: Some("db-1".into()),
            tab,
            ..Default::default()
        };
        let t = draw(&mut app, &fleet, 110, 28, None);
        insta::assert_snapshot!(
            format!(
                "host_tab_{}",
                tab.title()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .to_lowercase()
            ),
            t.backend()
        );
    }
}

#[test]
fn unreachable_host_details() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App {
        screen: Screen::Host,
        selected: Some("legacy-app".into()),
        ..Default::default()
    };
    let t = draw(&mut app, &fleet, 100, 14, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn security_pulse() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App {
        screen: Screen::Security,
        ..Default::default()
    };
    let t = draw(&mut app, &fleet, 130, 22, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn help_overlay() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App {
        help: true,
        ..Default::default()
    };
    let t = draw(&mut app, &fleet, 100, 30, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn history_mode_header_and_timeline() {
    let fleet = demo::fleet(now(), 0.0);
    let mut app = App::default();
    let t = draw(
        &mut app,
        &fleet,
        120,
        12,
        Some(now() - Duration::minutes(47)),
    );
    insta::assert_snapshot!(t.backend());
}

#[test]
fn empty_fleet() {
    let fleet = FleetState::default();
    let mut app = App::default();
    let t = draw(&mut app, &fleet, 60, 8, None);
    insta::assert_snapshot!(t.backend());
}

#[test]
fn word_wrap() {
    assert_eq!(
        ui::wrap_words("unknown host key for staging", 12),
        vec!["unknown host", "key for", "staging"]
    );
    assert_eq!(ui::wrap_words("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    assert!(ui::wrap_words("", 5).is_empty());
}

#[test]
fn bars() {
    assert_eq!(ui::bar(Some(0.0), 4), "····");
    assert_eq!(ui::bar(Some(100.0), 4), "████");
    assert_eq!(ui::bar(Some(50.0), 4), "██··");
    assert_eq!(ui::bar(Some(56.25), 4), "██▎·");
    assert_eq!(ui::bar(None, 3), "···");
    assert_eq!(ui::bar(Some(250.0), 2), "██");
}
