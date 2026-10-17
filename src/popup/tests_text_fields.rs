//! Purpose: Selection and the clipboard in the dialogs' text fields and the
//! command line: the keys, a terminal paste, and the selection drawn.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use super::{paste_text, process_popup_input};
use crate::clipboard::ClipboardBackend;
use crate::models::{App, FileName, NamePrompt, Overlay, Panel};

/// A clipboard in memory, shared with the test that installed it.
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);

impl ClipboardBackend for Memory {
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.0.borrow().clone())
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        text.clone_into(&mut self.0.borrow_mut());
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("no terminal in a test".to_owned())
    }
}

fn app_with(overlay: Overlay) -> (App, Memory) {
    let empty = || Panel::from_entries(PathBuf::from("/srv/x"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    let memory = Memory::default();
    app.clipboard.install(Box::new(memory.clone()));
    app.set_overlay(overlay);
    (app, memory)
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    process_popup_input(app, KeyEvent::new(code, modifiers));
}

fn rename_field(app: &App) -> &str {
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("the rename dialog is open");
    };
    prompt.name()
}

/// The report that started this: in the rename box, Shift selected nothing
/// and nothing could be copied.
#[test]
fn the_rename_field_selects_with_shift_and_copies() {
    let rename = NamePrompt::rename(FileName::new("report.txt"));
    let (mut app, memory) = app_with(Overlay::NamePrompt(rename));
    press(&mut app, KeyCode::End, KeyModifiers::NONE);
    for _ in 0..4 {
        press(&mut app, KeyCode::Left, KeyModifiers::SHIFT);
    }
    press(&mut app, KeyCode::Insert, KeyModifiers::CONTROL);
    assert_eq!(*memory.0.borrow(), ".txt");
    press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(rename_field(&app), "report");
    press(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(rename_field(&app), "report.txt.txt");
}

#[test]
fn a_terminal_paste_types_into_the_focused_field() {
    let rename = NamePrompt::rename(FileName::new("old.txt"));
    let (mut app, _memory) = app_with(Overlay::NamePrompt(rename));
    paste_text(&mut app, "new.txt\n");
    assert_eq!(rename_field(&app), "new.txt", "over the selected name");

    let (mut app, _memory) = app_with(Overlay::NewFileInput("a".into()));
    paste_text(&mut app, "b\r\nc");
    assert_eq!(app.get_new_file_input(), Some("ab c"));
}

#[test]
fn a_paste_with_no_field_under_the_cursor_changes_nothing() {
    let (mut app, _memory) = app_with(Overlay::Help);
    paste_text(&mut app, "text");
    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn the_command_line_selects_copies_cuts_and_pastes() {
    let (mut app, memory) = app_with(Overlay::None);
    app.set_overlay(Overlay::CommandLine(crate::models::CommandLine::default()));
    for ch in "echo hi".chars() {
        press(&mut app, KeyCode::Char(ch), KeyModifiers::NONE);
    }
    press(
        &mut app,
        KeyCode::Left,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    );
    press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(*memory.0.borrow(), "hi", "Ctrl+C copies a selection");
    assert_eq!(
        app.get_command_line_mut()
            .map(|line| line.text().to_owned())
            .as_deref(),
        Some("echo hi")
    );

    press(&mut app, KeyCode::Delete, KeyModifiers::SHIFT);
    press(&mut app, KeyCode::Home, KeyModifiers::NONE);
    press(&mut app, KeyCode::Insert, KeyModifiers::SHIFT);
    let line = app
        .get_command_line_mut()
        .map(|line| line.text().to_owned());
    assert_eq!(line.as_deref(), Some("hiecho "));

    // With nothing selected, Ctrl+C still abandons the line.
    press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
    let line = app
        .get_command_line_mut()
        .map(|line| line.text().to_owned());
    assert_eq!(line.as_deref(), Some(""));
}

/// The selection is drawn on the field's own cells, and nowhere else.
#[test]
fn a_selection_is_drawn_selected_in_the_field() {
    let rename = NamePrompt::rename(FileName::new("notes.txt"));
    let (mut app, _memory) = app_with(Overlay::NamePrompt(rename));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| crate::ui::ui(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let selected: String = buffer
        .content
        .iter()
        .filter(|cell| cell.bg == ratatui::style::Color::Rgb(0, 0, 0) && cell.symbol() != " ")
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(selected.contains("notes.txt"), "{selected:?}");
}
