//! A persistent production event loop; every input waits for the resulting frame.

use std::sync::{Arc, Mutex};
use std::{fs, io, path::PathBuf, sync::mpsc, thread, time::Duration};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    models::{App, Panel},
    run_app_with_event_reader,
};
use ratatui::{buffer::Buffer, layout::Size, Terminal};

use crate::editor_backend::SharedBackend;

const DEADLINE: Duration = Duration::from_secs(10);

enum Input {
    Event(Event),
    Stop,
}

enum Output {
    Frame(Buffer),
    Exited(io::Result<()>),
}

pub(super) struct Session {
    input: mpsc::Sender<Input>,
    output: mpsc::Receiver<Output>,
    current: Output,
    worker: Option<thread::JoinHandle<()>>,
    trace: Vec<String>,
    clipboard: Arc<Mutex<String>>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("frames", &self.trace.len())
            .finish_non_exhaustive()
    }
}

impl Session {
    pub(super) fn start(left: PathBuf, right: PathBuf, size: Size) -> Self {
        Self::start_app(
            move || Ok(App::with_panels(Panel::new(left)?, Panel::new(right)?)),
            size,
        )
    }

    pub(super) fn resume(directory: &std::path::Path, file: PathBuf, size: Size) -> Self {
        let directory = directory.to_owned();
        Self::start_app(
            move || {
                funny_file_manager::ops::panel_session::load(
                    &directory,
                    funny_file_manager::history::Persistence::File(file),
                )
            },
            size,
        )
    }

    fn start_app(
        create_app: impl FnOnce() -> io::Result<App> + Send + 'static,
        size: Size,
    ) -> Self {
        let (input, incoming) = mpsc::channel();
        let (outgoing, output) = mpsc::channel();
        let clipboard = Arc::new(Mutex::new(String::new()));
        let copied = Arc::clone(&clipboard);
        let worker = thread::spawn(move || {
            let result = create_app().and_then(|app| run(app, size, &incoming, &outgoing, copied));
            let _ = outgoing.send(Output::Exited(result));
        });
        let mut session = Self {
            input,
            output,
            current: Output::Exited(Ok(())),
            worker: Some(worker),
            trace: Vec::new(),
            clipboard,
        };
        session.receive("Initial screen");
        session
    }

    fn receive(&mut self, label: &str) {
        self.current = self
            .output
            .recv_timeout(DEADLINE)
            .expect("TUI response within 10 seconds");
        let text = match &self.current {
            Output::Frame(buffer) => buffer_text(buffer, 0..buffer.area.width),
            Output::Exited(result) => format!("Exited: {result:?}"),
        };
        self.trace.push(format!("{label}\n{text}\n"));
    }

    fn event(&mut self, event: Event) {
        assert!(
            matches!(self.current, Output::Frame(_)),
            "TUI already exited"
        );
        let label = format!("{event:?}");
        self.input.send(Input::Event(event)).expect("send input");
        self.receive(&label);
    }

    pub(super) fn press(&mut self, key: &str) {
        self.event(Event::Key(parse_key(key)));
    }

    pub(super) fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.event(Event::Key(KeyEvent::new(
                KeyCode::Char(ch),
                KeyModifiers::NONE,
            )));
        }
    }

    pub(super) fn resize(&mut self, size: Size) {
        self.event(Event::Resize(size.width, size.height));
    }

    pub(super) fn paste(&mut self, text: &str) {
        self.event(Event::Paste(text.to_owned()));
    }

    fn buffer(&self) -> &Buffer {
        match &self.current {
            Output::Frame(buffer) => buffer,
            Output::Exited(result) => panic!("expected a frame, got {result:?}"),
        }
    }

    pub(super) fn size(&self) -> Size {
        self.buffer().area.as_size()
    }

    pub(super) fn text(&self) -> String {
        buffer_text(self.buffer(), 0..self.size().width)
    }

    pub(super) fn clipboard(&self) -> String {
        self.clipboard.lock().expect("clipboard").clone()
    }

    pub(super) fn panel_text(&self, side: &str) -> String {
        let width = self.size().width;
        let columns = match side {
            "left" => 0..width / 2,
            "right" => width / 2..width,
            _ => panic!("unknown panel {side}"),
        };
        buffer_text(self.buffer(), columns)
    }

    pub(super) fn assert_exited(&self) {
        assert!(
            matches!(self.current, Output::Exited(Ok(()))),
            "session must exit normally"
        );
    }

    pub(super) fn stop(&mut self) {
        if matches!(self.current, Output::Frame(_)) && self.input.send(Input::Stop).is_ok() {
            if let Ok(output) = self.output.recv_timeout(DEADLINE) {
                self.current = output;
            }
        }
        if let Some(worker) = self.worker.take().filter(thread::JoinHandle::is_finished) {
            worker.join().expect("TUI worker panicked");
        }
    }

    pub(super) fn save_trace(&self, name: &str) -> io::Result<()> {
        let directory = std::env::var_os("FFM_TUI_ARTIFACT_DIR")
            .map_or_else(|| PathBuf::from("target/tui-bdd"), PathBuf::from);
        fs::create_dir_all(&directory)?;
        let name: String = name
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        fs::write(directory.join(format!("{name}.txt")), self.trace.join("\n"))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(
    mut app: App,
    size: Size,
    input: &mpsc::Receiver<Input>,
    output: &mpsc::Sender<Output>,
    clipboard: Arc<Mutex<String>>,
) -> io::Result<()> {
    app.clipboard.install(Box::new(MemoryClipboard(clipboard)));
    let backend = SharedBackend::new(size.width, size.height);
    let mut terminal = Terminal::new(backend.clone())?;
    run_app_with_event_reader(&mut terminal, app, || {
        output
            .send(Output::Frame(backend.buffer()))
            .map_err(io::Error::other)?;
        match input.recv_timeout(DEADLINE).map_err(io::Error::other)? {
            Input::Event(event) => {
                if let Event::Resize(width, height) = event {
                    backend.resize(Size::new(width, height));
                }
                Ok(event)
            }
            Input::Stop => Err(io::Error::new(io::ErrorKind::Interrupted, "BDD cleanup")),
        }
    })
}

struct MemoryClipboard(Arc<Mutex<String>>);

impl funny_file_manager::clipboard::ClipboardBackend for MemoryClipboard {
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.0.lock().expect("clipboard").clone())
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        text.clone_into(&mut self.0.lock().expect("clipboard"));
        Ok(())
    }
    fn emit_osc52(&mut self, _: &str) -> Result<(), String> {
        panic!("BDD must use its own clipboard")
    }
}

fn buffer_text(buffer: &Buffer, columns: std::ops::Range<u16>) -> String {
    (0..buffer.area.height)
        .map(|y| {
            columns
                .clone()
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_key(key: &str) -> KeyEvent {
    let (modifiers, key) = if let Some(key) = key.strip_prefix("Ctrl+") {
        (KeyModifiers::CONTROL, key)
    } else if let Some(key) = key.strip_prefix("Shift+") {
        (KeyModifiers::SHIFT, key)
    } else {
        (KeyModifiers::NONE, key)
    };
    KeyEvent::new(key_code(key, modifiers), modifiers)
}

fn key_code(key: &str, modifiers: KeyModifiers) -> KeyCode {
    match key {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" if modifiers == KeyModifiers::SHIFT => KeyCode::BackTab,
        "Tab" => KeyCode::Tab,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Delete" => KeyCode::Delete,
        "Backspace" => KeyCode::Backspace,
        "Space" => KeyCode::Char(' '),
        value if value.starts_with('F') => KeyCode::F(value[1..].parse().expect("function key")),
        value if value.chars().count() == 1 => KeyCode::Char(
            value
                .to_ascii_lowercase()
                .chars()
                .next()
                .expect("character"),
        ),
        _ => panic!("unknown key {key}"),
    }
}
