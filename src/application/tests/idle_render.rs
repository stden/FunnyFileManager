//! Polling must wake background jobs without repainting an unchanged screen.

use std::{
    io,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Size, Terminal};
use tempfile::TempDir;

use crate::{
    models::{App, Overlay, Panel, WslDialog, WslOperation, WslOutput},
    wsl::WslBackend,
};

use super::super::{run_app_with_event_reader, run_app_with_wsl_backend};

mod backend;
use backend::CountingBackend;

const WAIT_LIMIT: Duration = Duration::from_secs(5);

fn app_at(directory: &std::path::Path) -> io::Result<App> {
    let mut app = App::new()?;
    app.left_panel = Panel::new(directory.to_path_buf())?;
    app.right_panel = Panel::new(directory.to_path_buf())?;
    Ok(app)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn quit() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
}

struct QueuedText(std::collections::VecDeque<Event>);

impl super::input::EventSource for QueuedText {
    fn read(&mut self) -> io::Result<Option<Event>> {
        Ok(Some(self.0.pop_front().expect("script must quit")))
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        Ok(self.0.pop_front())
    }
}

#[test]
fn a_queued_paste_renders_once_before_saving_and_closing() -> io::Result<()> {
    let directory = TempDir::new()?;
    let path = directory.path().join("paste.txt");
    let mut app = app_at(directory.path())?;
    app.set_overlay(Overlay::Editor(crate::models::Editor::from_content(
        "paste.txt".to_owned(),
        path.clone(),
        "tail",
    )));
    let text = "Hello мир🙂\n".repeat(1_000);
    let events = text
        .chars()
        .map(|ch| {
            key(if ch == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(ch)
            })
        })
        .chain([key(KeyCode::F(2)), key(KeyCode::Esc), quit()]);
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    super::run_loop(
        &mut terminal,
        &mut app,
        QueuedText(events.collect()),
        Arc::new(NoWsl),
    )?;
    assert_eq!(std::fs::read_to_string(path)?, format!("{text}tail"));
    assert_eq!(screen.draws(), 4, "initial, paste, save, and panels only");
    Ok(())
}

struct NoWsl;

impl WslBackend for NoWsl {
    fn execute(&self, _: &WslOperation) -> Result<WslOutput, String> {
        panic!("this workflow must not launch a WSL operation")
    }
}

#[test]
fn idle_ticks_skip_drawing_while_keys_and_resize_repaint() -> io::Result<()> {
    let directory = TempDir::new()?;
    let app = app_at(directory.path())?;
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut script = idle_script();

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || {
            let (draws, event) = script.next().expect("script must quit");
            assert_eq!(screen.draws(), draws, "idle polls must not redraw");
            if let Some(Event::Resize(width, height)) = event.as_ref() {
                screen.resize(Size::new(*width, *height));
            }
            Ok(event)
        },
        Arc::new(NoWsl),
    )?;

    assert_eq!(screen.draws(), 3);
    assert_eq!(terminal.size()?, Size::new(34, 12));
    assert!(script.next().is_none());
    Ok(())
}

#[test]
fn legacy_event_reader_still_draws_after_each_event() -> io::Result<()> {
    let directory = TempDir::new()?;
    let app = app_at(directory.path())?;
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut script = [(1, key(KeyCode::Tab)), (2, key(KeyCode::Down)), (3, quit())].into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        let (draws, event) = script.next().expect("script must quit");
        assert_eq!(screen.draws(), draws);
        Ok(event)
    })?;
    assert_eq!(screen.draws(), 3);
    assert!(script.next().is_none());
    Ok(())
}

struct GatedWsl(Mutex<mpsc::Receiver<()>>);

impl WslBackend for GatedWsl {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        assert_eq!(*operation, WslOperation::Update);
        self.0
            .lock()
            .expect("completion gate")
            .recv_timeout(WAIT_LIMIT)
            .map_err(|error| error.to_string())?;
        Ok(WslOutput::Updated("Background update finished".to_owned()))
    }
}

#[test]
fn a_background_completion_repaints_once_without_a_key() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut app = app_at(directory.path())?;
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(Vec::new()))));
    crate::ops::wsl::update(&mut app);
    app.close_overlay();
    let (release, gate) = mpsc::channel();
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut polls = CompletionPolls::new(release);

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || polls.read(&screen),
        Arc::new(GatedWsl(Mutex::new(gate))),
    )?;

    assert_eq!(screen.draws(), 2);
    assert_eq!(polls.idle_after, 4);
    Ok(())
}

fn idle_script() -> impl Iterator<Item = (usize, Option<Event>)> {
    [
        (1, None),
        (1, None),
        (1, Some(key(KeyCode::Tab))),
        (2, None),
        (2, Some(Event::Resize(34, 12))),
        (3, None),
        (3, Some(quit())),
    ]
    .into_iter()
}
struct CompletionPolls {
    idle_before: usize,
    idle_after: usize,
    deadline: Instant,
    release: mpsc::Sender<()>,
}
impl CompletionPolls {
    fn new(release: mpsc::Sender<()>) -> Self {
        Self {
            idle_before: 0,
            idle_after: 0,
            deadline: Instant::now() + WAIT_LIMIT,
            release,
        }
    }
    fn read(&mut self, screen: &CountingBackend) -> io::Result<Option<Event>> {
        if self.idle_before < 4 {
            assert_eq!(screen.draws(), 1, "pending work must not redraw");
            self.idle_before += 1;
            if self.idle_before == 4 {
                self.release.send(()).expect("release background operation");
            }
            return Ok(None);
        }
        if screen.draws() == 1 {
            assert!(Instant::now() < self.deadline, "completion did not repaint");
            thread::yield_now();
            return Ok(None);
        }
        assert_eq!(screen.draws(), 2, "completed work must not keep redrawing");
        assert!(screen.text().contains("Background update finished"));
        self.idle_after += 1;
        Ok((self.idle_after == 4).then(quit))
    }
}
