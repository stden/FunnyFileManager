//! Queued navigation must yield frames without losing steps or crossing commands.

use std::{collections::VecDeque, io, sync::Arc};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;

use super::idle_render::backend::CountingBackend;
use crate::{
    application::input::EventSource,
    models::{App, Panel},
};

struct Queue(VecDeque<Event>);

impl EventSource for Queue {
    fn read(&mut self) -> io::Result<Option<Event>> {
        Ok(Some(self.0.pop_front().expect("script must quit")))
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        Ok(self.0.pop_front())
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn a_frame_is_presented_as_one_terminal_update() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    let mut app = App::with_panels(
        Panel::from_entries(directory.path().to_owned(), Vec::new()),
        Panel::from_entries(directory.path().to_owned(), Vec::new()),
    );
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    super::super::loop_state::draw_frame(&mut terminal, &mut app)?;
    assert_eq!(screen.writes(), b"\x1b[?2026h\x1b[?2026l");
    screen.fail_next_draw();
    assert_eq!(
        super::super::loop_state::draw_frame(&mut terminal, &mut app)
            .unwrap_err()
            .to_string(),
        "draw failed"
    );
    assert_eq!(
        screen.writes(),
        b"\x1b[?2026h\x1b[?2026l\x1b[?2026h\x1b[?2026l"
    );
    Ok(())
}

#[test]
fn queued_wheel_keeps_its_panel_target_and_draws_before_quit() -> io::Result<()> {
    use crossterm::event::{MouseEvent, MouseEventKind};
    let directory = tempfile::TempDir::new()?;
    for index in 0..80 {
        std::fs::write(directory.path().join(format!("file{index:03}")), "")?;
    }
    let mut app = App::with_panels(
        Panel::new(directory.path().to_owned())?,
        Panel::new(directory.path().to_owned())?,
    );
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let wheel = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 60,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    let events = std::iter::repeat_n(wheel, 64).chain([quit]);
    super::run_loop(
        &mut terminal,
        &mut app,
        Queue(events.collect()),
        Arc::new(crate::wsl::SystemWslBackend),
    )?;
    assert_eq!(app.left_panel.selected(), Some(0));
    assert_eq!(app.right_panel.selected(), Some(64));
    assert!(screen.draws() < 65);
    assert!(screen.text().contains("file063"));
    Ok(())
}

#[test]
fn queued_scroll_preserves_every_step_and_panel_switch() -> io::Result<()> {
    let directory = tempfile::TempDir::new()?;
    for index in 0..160 {
        std::fs::write(directory.path().join(format!("file{index:03}")), "")?;
    }
    let mut app = App::with_panels(
        Panel::new(directory.path().to_owned())?,
        Panel::new(directory.path().to_owned())?,
    );
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    let events = std::iter::repeat_n(key(KeyCode::Down), 128).chain([
        key(KeyCode::Tab),
        key(KeyCode::Down),
        quit,
    ]);
    super::run_loop(
        &mut terminal,
        &mut app,
        Queue(events.collect()),
        Arc::new(crate::wsl::SystemWslBackend),
    )?;
    assert_eq!(app.left_panel.selected(), Some(128));
    assert_eq!(app.right_panel.selected(), Some(1));
    assert!(screen.draws() < 131, "queued scroll should share frames");
    assert!(
        screen.draws() >= 5,
        "long bursts must yield intermediate frames"
    );
    Ok(())
}
