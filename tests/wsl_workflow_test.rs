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

const WAIT_LIMIT: Duration = Duration::from_secs(5);
const PLATFORM_DONE: &str = "Platform update done";
const PACKAGES_DONE: &str = "Debian packages updated";
const MENU_SELECTED: Color = Color::Rgb(0, 0, 0);
const PANEL_SELECTED: Color = Color::Rgb(85, 150, 216);

struct FakeWsl {
    operations: Mutex<Vec<WslOperation>>,
    root: PathBuf,
    release_update: Option<Mutex<mpsc::Receiver<()>>>,
}

impl FakeWsl {
    fn new(root: PathBuf) -> Self {
        Self {
            operations: Mutex::new(Vec::new()),
            root,
            release_update: None,
        }
    }

    fn operations(&self) -> Vec<WslOperation> {
        self.operations.lock().expect("recorded operations").clone()
    }
}

impl WslBackend for FakeWsl {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        self.operations
            .lock()
            .expect("record operations")
            .push(operation.clone());
        match operation {
            WslOperation::Refresh => Ok(WslOutput::Distributions(vec![
                distribution("Ubuntu"),
                distribution("Debian"),
            ])),
            WslOperation::Update => {
                if let Some(release) = &self.release_update {
                    release
                        .lock()
                        .expect("update release")
                        .recv_timeout(WAIT_LIMIT)
                        .map_err(|error| format!("test did not release the update: {error}"))?;
                }
                Ok(WslOutput::Updated(PLATFORM_DONE.to_owned()))
            }
            WslOperation::UpdatePackages(_) => {
                Ok(WslOutput::PackagesUpdated(PACKAGES_DONE.to_owned()))
            }
            WslOperation::Open(_) => Ok(WslOutput::Root(self.root.clone())),
        }
    }
}

fn distribution(name: &str) -> DistributionName {
    DistributionName::new(name).expect("valid fixture distribution")
}

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
    assert!(steps.is_empty(), "session must consume the complete script");
    Ok(())
}

#[test]
fn text_ui_updates_platform_and_selected_packages_then_opens_a_normal_panel() -> io::Result<()> {
    let root = TempDir::new()?;
    fs::write(root.path().join("root-visible.txt"), "distribution file")?;
    let backend = Arc::new(FakeWsl::new(root.path().to_path_buf()));
    run(backend.clone(), update_and_open_steps())?;
    assert_eq!(
        backend.operations(),
        [
            WslOperation::Refresh,
            WslOperation::Update,
            WslOperation::UpdatePackages(distribution("Debian")),
            WslOperation::Open(distribution("Debian")),
        ]
    );
    assert_eq!(
        fs::read_to_string(root.path().join("root-visible.txt"))?,
        "distribution file"
    );
    Ok(())
}

#[test]
fn text_ui_closes_a_busy_manager_keeps_keyboard_working_and_reports_completion() -> io::Result<()> {
    let root = TempDir::new()?;
    let (release, gate) = mpsc::channel();
    let mut backend = FakeWsl::new(root.path().to_path_buf());
    backend.release_update = Some(Mutex::new(gate));
    let backend = Arc::new(backend);
    run(
        backend.clone(),
        vec![
            open_manager(),
            Step::WaitFor("Debian"),
            key(KeyCode::F(5)),
            Step::WaitFor("Updating WSL..."),
            key(KeyCode::F(6)), // Busy state must suppress a second operation.
            key(KeyCode::Esc),
            Step::Inspect(assert_panels),
            key(KeyCode::Tab),
            key(KeyCode::Down),
            Step::Inspect(|buffer| assert_selected(buffer, "right-only", PANEL_SELECTED)),
            Step::ReleaseUpdate(release),
            Step::WaitFor(PLATFORM_DONE),
            Step::Inspect(assert_background_panels),
            key(KeyCode::F(10)),
            key(KeyCode::Enter),
        ],
    )?;
    assert_eq!(
        backend.operations(),
        [WslOperation::Refresh, WslOperation::Update]
    );
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
                release.send(()).expect("update worker is waiting");
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

fn assert_manager_controls(buffer: &ratatui::buffer::Buffer) {
    let frame = frame_text(buffer);
    for label in [
        "Enter Open root",
        "F2 Refresh list",
        "F5 Update WSL",
        "F6 Update packages",
    ] {
        assert!(frame.contains(label), "missing {label}:\n{frame}");
    }
    assert_selected(buffer, "Ubuntu", MENU_SELECTED);
}

fn assert_compact_manager(buffer: &ratatui::buffer::Buffer) {
    assert_selected(buffer, "Debian", MENU_SELECTED);
    let frame = frame_text(buffer);
    for label in ["Open", "F5", "F6", "Esc"] {
        assert!(
            frame.contains(label),
            "missing compact control {label}:\n{frame}"
        );
    }
}

fn assert_background_panels(buffer: &ratatui::buffer::Buffer) {
    assert_panels(buffer);
    assert_selected(buffer, "right-only", PANEL_SELECTED);
    assert!(frame_text(buffer).contains("left-only"));
}

fn update_and_open_steps() -> Vec<Step> {
    vec![
        open_manager(),
        Step::WaitFor("Debian"),
        Step::Inspect(assert_manager_controls),
        key(KeyCode::Down),
        Step::Resize(Size::new(18, 6)),
        Step::Inspect(assert_compact_manager),
        Step::Resize(Size::new(90, 20)),
        key(KeyCode::F(5)),
        Step::WaitFor(PLATFORM_DONE),
        key(KeyCode::F(6)),
        Step::WaitFor(PACKAGES_DONE),
        key(KeyCode::Enter),
        // Panel names and extensions occupy separate columns.
        Step::WaitFor("root-visible"),
        Step::Inspect(assert_panels),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ]
}
