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
fn esc_closes_a_clean_dialog_at_once() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.overlay, Overlay::None));
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(10));
    assert!(matches!(app.overlay, Overlay::None));
}

/// Space toggles; Esc on a changed dialog asks, and the exit question has the
/// editor's keys.
#[test]
fn a_changed_dialog_asks_before_closing() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(editor(&app).draft().0["bat"].enabled, Enablement::Disabled);
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Save));
    press(&mut app, KeyCode::Right);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Discard));
    press(&mut app, KeyCode::Tab);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Cancel));
    press(&mut app, KeyCode::Left);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Discard));
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(mode(&app), &Mode::Browse, "Cancel returns to the list");
    press(&mut app, KeyCode::F(10));
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse, "Esc is Cancel too");
}

#[test]
fn no_discards_the_changes_and_writes_nothing() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.overlay, Overlay::None));
    assert!(matches!(
        app.status,
        crate::status::Status::ChangesDiscarded
    ));
    assert!(!machine.settings().exists());
}

#[test]
fn yes_saves_and_closes_and_f2_saves_in_place() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::F(2));
    assert!(machine.settings().exists());
    assert!(!editor(&app).dirty());
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.overlay, Overlay::None), "Enter on Yes saves");
    assert_eq!(app.config.lsp.0["bat"].enabled, Enablement::Enabled);
}

/// A save the draft cannot make returns to the list with the reason.
#[test]
fn a_refused_save_from_the_exit_question_returns_to_the_list() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    select(&mut app, "python");
    press(&mut app, KeyCode::F(4));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(focus(&mut app), FormField::Extensions);
    press(&mut app, KeyCode::End);
    type_text(&mut app, " ts");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        mode(&app),
        &Mode::Browse,
        "Enter on a passive list presses OK"
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::ConfirmExit(ExitChoice::Save));
    press(&mut app, KeyCode::Char('y'));
    assert_eq!(mode(&app), &Mode::Browse);
    assert!(matches!(
        editor(&app).notice(),
        Some(Notice::Error(message)) if message.starts_with("Not saved: .ts is claimed by")
    ));
    assert!(!machine.settings().exists());
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

#[test]
fn del_resets_a_catalog_server_and_removes_an_added_one() {
    let machine = LspMachine::new();
    machine.write_settings("lsp:\n  css: {command: c}\n  mine: {command: m}\n");
    let mut app = machine.opened();
    select(&mut app, "css");
    press(&mut app, KeyCode::Delete);
    assert_eq!(
        editor(&app).draft().0["css"].command,
        "vscode-css-language-server"
    );
    select(&mut app, "mine");
    press(&mut app, KeyCode::F(8));
    assert!(!editor(&app).draft().0.contains_key("mine"));
    press(&mut app, KeyCode::F(5));
    assert!(matches!(editor(&app).notice(), Some(Notice::Info(_))));
}

/// Del on the added `zig`, then Ins with `command` and extension `zig`.
fn remove_and_add_zig_again(app: &mut App, command: &str) {
    select(app, "zig");
    press(app, KeyCode::Delete);
    press(app, KeyCode::Insert);
    type_text(app, "zig");
    press(app, KeyCode::Tab);
    type_text(app, command);
    press(app, KeyCode::Tab);
    press(app, KeyCode::Tab);
    type_text(app, "zig");
    press(app, KeyCode::Enter);
    assert_eq!(mode(app), &Mode::Browse, "{command}: the form closed");
}

/// Del then Ins on an added server's language makes a new server: F2 writes
/// it whole, and re-entering the old command and extension still saves.
#[test]
fn a_removed_server_added_again_is_saved_as_the_new_one() {
    for command in ["zls2", "zls"] {
        let machine = LspMachine::new();
        machine.write_settings(
            "lsp:\n  zig: {command: zls, extensions: [zig], root_markers: [build.zig], custom: 1}\n",
        );
        let mut app = machine.opened();
        remove_and_add_zig_again(&mut app, command);
        press(&mut app, KeyCode::F(2));
        assert!(
            !editor(&app).dirty(),
            "{command}: {:?}",
            editor(&app).notice()
        );
        let file = machine.parsed();
        let zig = &file["lsp"]["zig"];
        assert_eq!(zig["command"], serde_norway::Value::from(command));
        assert_eq!(
            zig["root_markers"],
            serde_norway::Value::Sequence(vec![".git".into()])
        );
        assert!(zig.get("custom").is_none(), "{file:?}");
    }
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
