//! Real WSL text-UI workflows with an injected backend; no native WSL commands.

use std::{
    collections::VecDeque,
    fs, io,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    models::{App, DistributionName, Panel, WslOperation, WslOutput},
    run_app_with_wsl_backend,
    wsl::WslBackend,
};
use ratatui::{buffer::Buffer, layout::Size, style::Color, Terminal};
use tempfile::TempDir;

#[path = "support/editor_backend.rs"]
mod editor_backend;
use editor_backend::SharedBackend;

#[path = "wsl_workflow_test/busy_manager.rs"]
mod busy_manager;
#[path = "wsl_workflow_test/fake_wsl.rs"]
mod fake_wsl;
#[path = "wsl_workflow_test/update_and_open.rs"]
mod update_and_open;
use fake_wsl::{distribution, FakeWsl};

const WAIT_LIMIT: Duration = Duration::from_secs(5);
const PLATFORM_DONE: &str = "Platform update done";
const PACKAGES_DONE: &str = "Debian packages updated";

enum Step {
    Key(KeyCode, KeyModifiers),
    WaitFor(&'static str),
    Inspect(fn(&Buffer)),
    Resize(Size),
    ReleaseUpdate(mpsc::Sender<()>),
}

fn key(code: KeyCode) -> Step {
    Step::Key(code, KeyModifiers::NONE)
}

fn open_manager() -> Step {
    Step::Key(
        KeyCode::Char('w'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    )
}

fn frame_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_selected(buffer: &Buffer, name: &str, background: Color) {
    let selected = (0..buffer.area.height).any(|y| {
        (0..buffer.area.width).any(|x| {
            buffer[(x, y)].bg == background
                && (x..buffer.area.width)
                    .map(|column| buffer[(column, y)].symbol())
                    .collect::<String>()
                    .starts_with(name)
        })
    });
    assert!(
        selected,
        "{name} must be visibly selected:\n{}",
        frame_text(buffer)
    );
}

fn assert_panels(buffer: &Buffer) {
    let frame = frame_text(buffer);
    assert!(frame.contains("Left") && frame.contains("Right"), "{frame}");
    assert!(
        !frame.contains("Enter Open root"),
        "WSL dialog must be closed:\n{frame}"
    );
}

fn waiting_for_frame(
    step: Option<&Step>,
    screen: &SharedBackend,
    deadline: Instant,
) -> io::Result<bool> {
    let Some(Step::WaitFor(expected)) = step else {
        return Ok(false);
    };
    let frame = frame_text(&screen.buffer());
    if frame.contains(*expected) {
        return Ok(false);
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("waiting for {expected:?} in rendered frame:\n{frame}"),
        ));
    }
    thread::sleep(Duration::from_millis(1));
    Ok(true)
}

fn run(backend: Arc<dyn WslBackend>, script: Vec<Step>) -> io::Result<()> {
    let left = TempDir::new()?;
    let right = TempDir::new()?;
    fs::write(left.path().join("left-only.txt"), "left")?;
    fs::write(right.path().join("right-only.txt"), "right")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(left.path().to_path_buf())?;
    app.right_panel = Panel::new(right.path().to_path_buf())?;

    let screen = SharedBackend::new(90, 20);
    let mut terminal = Terminal::new(screen.clone())?;
    let mut steps = VecDeque::from(script);
    let mut deadline = Instant::now() + WAIT_LIMIT;
    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || read_step(&mut steps, &screen, &mut deadline),
        backend,
    )?;
    assert_eq!(steps.len(), 0, "session must consume the complete script");
    Ok(())
}

fn read_step(
    steps: &mut VecDeque<Step>,
    screen: &SharedBackend,
    deadline: &mut Instant,
) -> io::Result<Option<Event>> {
    loop {
        if waiting_for_frame(steps.front(), screen, *deadline)? {
            return Ok(None);
        }
        *deadline = Instant::now() + WAIT_LIMIT;
        match steps.pop_front() {
            Some(Step::Key(code, modifiers)) => {
                return Ok(Some(Event::Key(KeyEvent::new(code, modifiers))));
            }
            Some(Step::WaitFor(_)) => {}
            Some(Step::Inspect(inspect)) => inspect(&screen.buffer()),
            Some(Step::Resize(size)) => {
                screen.resize(size);
                return Ok(Some(Event::Resize(size.width, size.height)));
            }
            Some(Step::ReleaseUpdate(release)) => {
                release.send(()).map_err(|error| {
                    io::Error::other(format!("update worker is waiting: {error}"))
                })?;
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "script did not quit",
                ));
            }
        }
    }
}
