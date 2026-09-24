//! Complete text-UI workflows: keys, rendered frames, clipboard and saved file.

use std::{cell::RefCell, fs, io, rc::Rc};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    clipboard::ClipboardBackend,
    models::{App, Panel},
    run_app_with_event_reader,
};
use ratatui::{buffer::Buffer, layout::Size, style::Color, Terminal};
use tempfile::TempDir;

#[path = "support/editor_backend.rs"]
mod editor_backend;
use editor_backend::SharedBackend;

#[path = "editor_workflow_test/clipboard.rs"]
mod clipboard;
#[path = "editor_workflow_test/dotenv_session.rs"]
mod dotenv_session;
#[path = "editor_workflow_test/selection_clipboard.rs"]
mod selection_clipboard;
use clipboard::Clipboard;

const ORIGINAL: &str = "GREETING=\"a界\"\r\nNEXT=\"β🙂\"\r\nPORT=3000\r\n";
const COPIED: &str = "GREETING=\"a界\"\r\nNEXT=\"β🙂\"\r\n";
const REPLACEMENT: &str = "GREETING=\"new界\"\r\nNEXT=\"fresh🙂\"\r\n";
const SAVED: &str = "GREETING=\"new界\"\r\nNEXT=\"fresh🙂\"\r\nPORT=3000\r\n";
const BLUE: Color = Color::Rgb(0, 0, 128);
const SELECTED: Color = Color::Rgb(85, 150, 216);
const ENV_KEY: Color = Color::Rgb(140, 220, 200);
const ENV_VALUE: Color = Color::Rgb(235, 185, 70);

enum Step {
    Key(KeyCode, KeyModifiers),
    Paste(String),
    Inspect(fn(&Buffer)),
    Clipboard(&'static str),
    Resize(Size),
}

fn key(code: KeyCode) -> Step {
    Step::Key(code, KeyModifiers::NONE)
}

fn shifted(code: KeyCode) -> Step {
    Step::Key(code, KeyModifiers::SHIFT)
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

/// Open by F4, then select the first two lines upwards from the third line.
fn open_and_select() -> Vec<Step> {
    vec![
        key(KeyCode::Down), // `..` -> .env in the filesystem listing.
        key(KeyCode::F(4)),
        Step::Inspect(assert_original_buffer),
        key(KeyCode::Down),
        key(KeyCode::Down),
        shifted(KeyCode::Up),
        shifted(KeyCode::Up),
        Step::Inspect(assert_selection),
    ]
}

fn assert_selection(buffer: &Buffer) {
    assert_eq!(buffer[(0, 1)].bg, SELECTED);
    assert_eq!(buffer[(11, 1)].symbol(), "界");
    assert_eq!(buffer[(11, 1)].bg, SELECTED);
    assert_eq!(buffer[(7, 2)].symbol(), "🙂");
    assert_eq!(buffer[(7, 2)].bg, SELECTED);
    assert_eq!(buffer[(0, 3)].symbol(), "P");
    assert_eq!(buffer[(0, 3)].bg, BLUE, "third line is outside selection");
}

fn finish() -> Vec<Step> {
    vec![
        key(KeyCode::F(2)),
        key(KeyCode::Esc),
        Step::Inspect(|buffer| {
            assert!(
                row(buffer, 0).contains("Left")
                    && (0..buffer.area.height).any(|y| row(buffer, y).contains(".env")),
                "editor must close back to panels"
            );
        }),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ]
}

fn run(directory: &TempDir, clipboard: &Clipboard, steps: Vec<Step>) -> io::Result<()> {
    let other = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(directory.path().to_path_buf())?;
    app.right_panel = Panel::new(other.path().to_path_buf())?;
    app.clipboard.install(Box::new(clipboard.clone()));
    let backend = SharedBackend::new(90, 20);
    let mut terminal = Terminal::new(backend.clone())?;
    let mut steps = steps.into_iter();
    run_app_with_event_reader(&mut terminal, app, || {
        read_step(&mut steps, &backend, clipboard)
    })?;
    assert!(
        steps.next().is_none(),
        "session ended before consuming the script"
    );
    assert!(
        other.path().read_dir()?.next().is_none(),
        "editor shortcuts must not copy panel files"
    );
    Ok(())
}

fn read_step(
    steps: &mut impl Iterator<Item = Step>,
    backend: &SharedBackend,
    clipboard: &Clipboard,
) -> io::Result<Event> {
    loop {
        match steps.next() {
            Some(Step::Paste(text)) => return Ok(Event::Paste(text)),
            Some(Step::Key(code, modifiers)) => {
                return Ok(Event::Key(KeyEvent::new(code, modifiers)));
            }
            Some(Step::Inspect(check)) => check(&backend.buffer()),
            Some(Step::Clipboard(text)) => text.clone_into(&mut clipboard.0.borrow_mut().text),
            Some(Step::Resize(size)) => {
                backend.resize(size);
                return Ok(Event::Resize(size.width, size.height));
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "script did not quit",
                ))
            }
        }
    }
}

fn assert_pasted_buffer(buffer: &Buffer) {
    assert_highlighted_buffer(buffer);
    assert_eq!(buffer[(13, 1)].symbol(), "界");
}

fn assert_original_buffer(buffer: &Buffer) {
    assert_highlighted_buffer(buffer);
    assert_eq!(buffer[(11, 1)].symbol(), "界");
}

fn assert_highlighted_buffer(buffer: &Buffer) {
    assert!(row(buffer, 1).contains("GREETING="));
    assert_eq!(buffer[(0, 1)].fg, ENV_KEY);
    assert_eq!(buffer[(10, 1)].fg, ENV_VALUE);
}
