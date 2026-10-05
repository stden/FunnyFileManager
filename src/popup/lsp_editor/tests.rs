//! Purpose: Pin every key of the Language servers dialog, mode by mode, as
//! the parity doc lists them.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::config::Enablement;
use crate::menu::SettingsPage;
use crate::models::lsp_editor::{Drawn, FormField, ListFocus, Mode, Notice};
use crate::models::{App, ExitChoice, Overlay};
use crate::scroll::ScrollTop;
use crate::test_support::lsp_machine::{
    editor, editor_mut, press, press_with, select, type_text, LspMachine,
};

mod exit_question;
mod servers;

fn mode(app: &App) -> &Mode {
    editor(app).mode()
}

fn selected(app: &App) -> &str {
    editor(app).selected().expect("a selection").as_str()
}

fn focus(app: &mut App) -> FormField {
    editor_mut(app).form_mut().expect("form").focus()
}

fn field(app: &mut App, field: FormField) -> String {
    let form = editor_mut(app).form_mut().expect("form");
    form.line(field).expect("text").text().to_owned()
}

fn list_focus(app: &mut App) -> Option<ListFocus> {
    let form = editor_mut(app).form_mut().expect("form");
    form.completion()
        .map(crate::models::lsp_editor::Suggestions::focus)
}

#[test]
fn the_arrows_and_paging_keys_move_the_selection() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Down);
    assert_eq!(selected(&app), "css");
    press(&mut app, KeyCode::End);
    assert_eq!(selected(&app), "yaml");
    press(&mut app, KeyCode::Up);
    assert_eq!(selected(&app), "typescriptreact");
    press(&mut app, KeyCode::Home);
    assert_eq!(selected(&app), "bat");
    editor_mut(&mut app).record_drawn(Drawn::List { page: 3 });
    press(&mut app, KeyCode::PageDown);
    assert_eq!(selected(&app), "java");
    press(&mut app, KeyCode::PageUp);
    assert_eq!(selected(&app), "bat");
}

#[test]
fn f3_shows_every_server_and_scrolls_it() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(3));
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP));
    editor_mut(&mut app).record_drawn(Drawn::View {
        lines: 40,
        page: 10,
    });
    press(&mut app, KeyCode::Down);
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP.down(1, 30)));
    press(&mut app, KeyCode::PageDown);
    press(&mut app, KeyCode::End);
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP.down(30, 30)));
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::PageUp);
    press(&mut app, KeyCode::Up);
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP));
    press(&mut app, KeyCode::F(5));
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("Checked every server again.".to_owned()))
    );
    press(&mut app, KeyCode::F(3));
    assert_eq!(mode(&app), &Mode::Browse);
    assert_eq!(selected(&app), "bat", "the selection is unchanged");
}

/// The form and the exit question open centred, however far the last
/// one was dragged, as every new dialog does.
#[test]
fn the_form_and_the_exit_question_open_centred() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Enter);
    app.dialog_mouse.offset = (-10, 0);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(mode(&app), Mode::Form(_)));
    assert_eq!(app.dialog_mouse.offset, (0, 0), "the form");
    app.dialog_mouse.offset = (-10, 0);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    assert!(matches!(mode(&app), Mode::ConfirmExit(_)));
    assert_eq!(app.dialog_mouse.offset, (0, 0), "the exit question");
}

/// Keys not listed do nothing: with a modifier, a list or view key is a
/// different key (Shift+F8, Alt+F7 and Shift+F2 mean other things in Far).
#[test]
fn a_modified_list_or_view_key_does_nothing() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let draft = editor(&app).draft().clone();
    let modified = [
        (KeyCode::F(8), KeyModifiers::SHIFT),
        (KeyCode::F(7), KeyModifiers::ALT),
        (KeyCode::F(2), KeyModifiers::SHIFT),
        (KeyCode::Char(' '), KeyModifiers::CONTROL),
        (KeyCode::Enter, KeyModifiers::ALT),
        (KeyCode::Down, KeyModifiers::CONTROL),
        (KeyCode::Delete, KeyModifiers::SHIFT),
        (KeyCode::F(3), KeyModifiers::SHIFT),
    ];
    for (code, modifiers) in modified {
        press_with(&mut app, code, modifiers);
        assert_eq!(mode(&app), &Mode::Browse, "{modifiers:?}+{code:?}");
        assert_eq!(editor(&app).draft(), &draft, "{modifiers:?}+{code:?}");
        assert_eq!(selected(&app), "bat", "{modifiers:?}+{code:?}");
    }
    assert!(!machine.settings().exists(), "nothing saved");
    press(&mut app, KeyCode::F(3));
    press_with(&mut app, KeyCode::F(3), KeyModifiers::SHIFT);
    press_with(&mut app, KeyCode::Esc, KeyModifiers::ALT);
    assert!(matches!(mode(&app), Mode::View(_)));
}

#[path = "tests_form.rs"]
mod form;
