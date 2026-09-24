//! Purpose: The task list's refresh through the real event loop: an open,
//! idle list draws once per sample applied and not between them, and a
//! closed one never starts a sampling thread or draws for one.

use std::io;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use tempfile::TempDir;

use super::super::run_app_with_wsl_backend;
use super::idle_render::backend::CountingBackend;
use crate::models::{App, Overlay, Panel, Pid, TaskList, WslOperation, WslOutput};
use crate::prompt::Privilege;
use crate::test_support::processes::{one_core, process, FixtureHost, Gate};
use crate::wsl::WslBackend;

const WAIT: Duration = Duration::from_secs(5);

struct NoWsl;

impl WslBackend for NoWsl {
    fn execute(&self, _: &WslOperation) -> Result<WslOutput, String> {
        panic!("the task list never runs a WSL operation")
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Option<Event> {
    Some(Event::Key(KeyEvent::new(code, modifiers)))
}

/// Panels on an empty directory, with `host` installed.
fn app(root: &TempDir, host: &FixtureHost) -> App {
    let panel = || Panel::from_entries(root.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel(), panel());
    app.task_host.install(Arc::new(host.clone()));
    app
}

/// `app` with the task list open, as a new session.
fn with_task_list(mut app: App) -> App {
    let session = app.task_host.next_session();
    let own = Pid::new(900).expect("a PID");
    app.set_overlay(Overlay::Tasks(Box::new(TaskList::new(
        session,
        own,
        Privilege::User,
    ))));
    app
}

/// The idle polls of an open list: nothing to draw until the gate lets the
/// first sample through, one frame for it, and nothing after while the
/// next sample waits at the gate. Then Esc closes the list and Ctrl+C quits.
struct SamplePolls {
    gate: Gate,
    before: usize,
    after: usize,
    deadline: Instant,
}

impl SamplePolls {
    const IDLE: usize = 5;

    fn read(&mut self, screen: &CountingBackend) -> Option<Event> {
        if self.before < Self::IDLE {
            assert_eq!(screen.draws(), 1, "no sample yet, no frame");
            self.before += 1;
            if self.before == Self::IDLE {
                let firefox = process(Pid::new(42).expect("a PID"), "firefox");
                self.gate.open(Ok(one_core(vec![firefox])));
            }
            return None;
        }
        if screen.draws() == 1 {
            assert!(Instant::now() < self.deadline, "the sample was not drawn");
            thread::yield_now();
            return None;
        }
        self.after += 1;
        if self.after > Self::IDLE {
            return key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        }
        assert_eq!(screen.draws(), 2, "one frame for one sample");
        (self.after == Self::IDLE)
            .then(|| key(KeyCode::Esc, KeyModifiers::NONE))
            .flatten()
    }
}

#[test]
fn an_idle_open_task_list_draws_once_per_sample_applied() -> io::Result<()> {
    let root = TempDir::new()?;
    let (host, gate) = FixtureHost::gated();
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut polls = SamplePolls {
        gate,
        before: 0,
        after: 0,
        deadline: Instant::now() + WAIT,
    };
    run_app_with_wsl_backend(
        &mut terminal,
        with_task_list(app(&root, &host)),
        || Ok(polls.read(&screen)),
        Arc::new(NoWsl),
    )?;
    assert_eq!(
        screen.draws(),
        3,
        "the first frame, the sample, and closing"
    );
    assert_eq!(host.readers_opened(), 1);
    Ok(())
}

#[test]
fn a_closed_task_list_starts_no_thread_and_draws_nothing() -> io::Result<()> {
    let root = TempDir::new()?;
    let (host, gate) = FixtureHost::gated();
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut polls = 0;
    run_app_with_wsl_backend(
        &mut terminal,
        app(&root, &host),
        || {
            polls += 1;
            assert_eq!(screen.draws(), 1, "idle polls never draw");
            Ok((polls > 10)
                .then(|| key(KeyCode::Char('c'), KeyModifiers::CONTROL))
                .flatten())
        },
        Arc::new(NoWsl),
    )?;
    assert_eq!((host.readers_opened(), gate.entered()), (0, 0));
    Ok(())
}
