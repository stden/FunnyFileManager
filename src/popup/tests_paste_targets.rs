//! Purpose: A terminal paste lands in the path editor, a session field, or an
//! attribute time, and nowhere else in those dialogs.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::{paste_text, text_field};
use crate::clipboard::ClipboardBackend;
use crate::models::{App, AttributesField, Overlay, Panel, SessionEdit};
use crate::path_env::{Mode, PathEditor, Scope, Snapshot};

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

fn app_with(overlay: Overlay) -> App {
    let empty = || Panel::from_entries(PathBuf::from("/srv/x"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    app.set_overlay(overlay);
    app
}

#[test]
fn a_paste_in_path_edit_mode_lands_in_the_line() {
    let editor = PathEditor::new(
        Scope::User,
        Snapshot {
            value: String::new(),
            revision: String::new(),
            location: "fixture only".to_owned(),
        },
    );
    let mut app = app_with(Overlay::PathEditor(Box::new(editor)));
    assert!(app.focused_text_field().is_none());
    paste_text(&mut app, "ignored");
    let Overlay::PathEditor(editor) = &mut app.overlay else {
        panic!("path dialog");
    };
    editor.edit(None);
    assert_eq!(editor.focused_line_mut().expect("editing").text(), "");
    paste_text(&mut app, "a\r\nb");
    let Overlay::PathEditor(editor) = &app.overlay else {
        panic!("path dialog");
    };
    let Mode::Edit { input, .. } = &editor.mode else {
        panic!("still editing");
    };
    assert_eq!(input.text(), "a b");
}

#[test]
fn a_paste_fills_the_session_field_the_cursor_is_on() {
    let mut app = app_with(Overlay::SessionEdit(Box::new(SessionEdit::create())));
    paste_text(&mut app, "bob\n");
    let Overlay::SessionEdit(dialog) = &mut app.overlay else {
        panic!("session");
    };
    assert_eq!(dialog.name, "bob");
    dialog.insert_text("-2");
    assert_eq!(dialog.name, "bob-2");
    let protocol = dialog.protocol;
    dialog.tab(true);
    paste_text(&mut app, "nope");
    let Overlay::SessionEdit(dialog) = &mut app.overlay else {
        panic!("session");
    };
    assert_eq!(dialog.protocol, protocol);
    assert_eq!(dialog.name, "bob-2");
    dialog.tab(true);
    paste_text(&mut app, "example.com\r\n");
    let Overlay::SessionEdit(dialog) = &app.overlay else {
        panic!("session");
    };
    assert_eq!(dialog.host, "example.com");
    assert_eq!(dialog.user, "");
}

#[test]
fn a_paste_fills_only_the_attribute_time_under_the_cursor() {
    let mut app = app_with(Overlay::Attributes(Box::default()));
    paste_text(&mut app, "owner");
    let Overlay::Attributes(dialog) = &mut app.overlay else {
        panic!("attributes");
    };
    assert_eq!(dialog.owner, "");
    assert_eq!(dialog.modified.current, "");
    dialog.field = AttributesField::Modified;
    paste_text(&mut app, "2026-10-01\n12:00");
    let Overlay::Attributes(dialog) = &app.overlay else {
        panic!("attributes");
    };
    assert_eq!(dialog.modified.current, "2026-10-01 12:00");
    assert_eq!(dialog.accessed.current, "");
}

#[test]
fn with_clipboard_lends_the_slot_and_puts_it_back() {
    let mut app = app_with(Overlay::None);
    let memory = Memory::default();
    app.clipboard.install(Box::new(memory));
    text_field::with_clipboard(&mut app, |app, slot| {
        assert!(app.clipboard.backend_mut().is_none());
        slot.backend_mut()
            .expect("lent")
            .set_native("kept")
            .expect("set");
    });
    let restored = app.clipboard.backend_mut().expect("returned");
    assert_eq!(restored.get_native().expect("read"), "kept");
}
