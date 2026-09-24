//! Purpose: Pin the form's keys and both states of its suggestion list.

use super::*;

fn edit_python(machine: &LspMachine) -> App {
    let mut app = machine.opened();
    select(&mut app, "python");
    press(&mut app, KeyCode::Enter);
    assert!(matches!(mode(&app), Mode::Form(_)));
    app
}

#[test]
fn tab_and_the_arrows_walk_the_controls_and_skip_the_language() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    assert_eq!(focus(&mut app), FormField::Command);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(
        focus(&mut app),
        FormField::Cancel,
        "wraps past the language"
    );
    press(&mut app, KeyCode::Down);
    assert_eq!(focus(&mut app), FormField::Command);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(focus(&mut app), FormField::Enabled);
    press(&mut app, KeyCode::Up);
    assert_eq!(focus(&mut app), FormField::Extensions);
}

#[test]
fn space_toggles_enabled_and_presses_a_button() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    for _ in 0..3 {
        press(&mut app, KeyCode::Tab);
    }
    press(&mut app, KeyCode::Char(' '));
    let form = editor_mut(&mut app).form_mut().expect("form");
    assert_eq!(form.enabled(), Enablement::Disabled);
    press(&mut app, KeyCode::Tab);
    assert_eq!(focus(&mut app), FormField::Ok);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(mode(&app), &Mode::Browse, "Space on OK presses it");
    assert_eq!(
        editor(&app).draft().0["python"].enabled,
        Enablement::Disabled
    );
}

#[test]
fn enter_on_cancel_and_esc_drop_the_forms_changes() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    type_text(&mut app, "-x");
    press(&mut app, KeyCode::BackTab);
    press(&mut app, KeyCode::Enter);
    assert_eq!(mode(&app), &Mode::Browse);
    assert!(!editor(&app).dirty());

    press(&mut app, KeyCode::F(4));
    type_text(&mut app, "-x");
    assert_eq!(list_focus(&mut app), None, "nothing matches pylsp-x");
    press(&mut app, KeyCode::Esc);
    assert_eq!(mode(&app), &Mode::Browse);
    assert!(!editor(&app).dirty());
}

#[test]
fn ins_then_ok_adds_a_server() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    app.settings
        .set(SettingsPage::AutoComplete, "off")
        .expect("valid");
    press(&mut app, KeyCode::Insert);
    assert_eq!(focus(&mut app), FormField::Language);
    type_text(&mut app, "zig");
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "zls");
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    type_text(&mut app, "zig zon");
    press(&mut app, KeyCode::Enter);
    assert_eq!(mode(&app), &Mode::Browse);
    assert_eq!(selected(&app), "zig");
    assert_eq!(editor(&app).draft().0["zig"].extensions, ["zig", "zon"]);
}

/// Every text key edits the focused field; a refused OK keeps the form.
#[test]
fn text_keys_edit_the_field_and_a_bad_value_keeps_the_form() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    app.settings
        .set(SettingsPage::AutoComplete, "off")
        .expect("valid");
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Left);
    type_text(&mut app, "~");
    assert_eq!(field(&mut app, FormField::Command), "ls~p");
    press(&mut app, KeyCode::Home);
    type_text(&mut app, "~");
    press(&mut app, KeyCode::Enter);
    assert!(matches!(mode(&app), Mode::Form(_)));
    let form = editor_mut(&mut app).form_mut().expect("form");
    assert_eq!(
        form.help(),
        "No shell runs the server, so ~ is not expanded; write the full path."
    );
}

#[test]
fn ctrl_space_ctrl_down_and_alt_down_open_an_active_list() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        let machine = LspMachine::new();
        let mut app = edit_python(&machine);
        press(&mut app, KeyCode::Tab);
        press_with(&mut app, KeyCode::Down, modifiers);
        assert_eq!(list_focus(&mut app), Some(ListFocus::List), "{modifiers:?}");
    }
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    press(&mut app, KeyCode::Tab);
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert_eq!(list_focus(&mut app), Some(ListFocus::List));
    assert_eq!(field(&mut app, FormField::Arguments), "", "no space typed");
}

/// A list opened by typing is passive: Enter still presses OK. Down enters it,
/// and Enter then takes the highlighted suggestion.
#[test]
fn a_passive_list_leaves_enter_to_the_form_until_down_enters_it() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "rust-an");
    assert_eq!(list_focus(&mut app), Some(ListFocus::Field));
    press(&mut app, KeyCode::Down);
    assert_eq!(list_focus(&mut app), Some(ListFocus::List));
    press(&mut app, KeyCode::Enter);
    assert_eq!(field(&mut app, FormField::Command), "rust-analyzer");
    assert_eq!(list_focus(&mut app), None);
    assert!(
        matches!(mode(&app), Mode::Form(_)),
        "accepting keeps the form"
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(mode(&app), &Mode::Browse);
    assert_eq!(editor(&app).draft().0["python"].command, "rust-analyzer");
}

/// With `AutoComplete` off typing opens nothing, but an open list follows
/// every edit; Left, Right, Esc and Tab close it.
#[test]
fn an_open_list_follows_typing_and_closes_on_caret_moves() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    app.settings
        .set(SettingsPage::AutoComplete, "off")
        .expect("valid");
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "vs");
    assert_eq!(list_focus(&mut app), None, "AutoComplete is off");
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert_eq!(list_focus(&mut app), Some(ListFocus::List));
    press(&mut app, KeyCode::Up);
    type_text(&mut app, "code-j");
    let form = editor_mut(&mut app).form_mut().expect("form");
    let list = form.completion().expect("refiltered, still open");
    assert_eq!(list.menu().selection().text, "vscode-json-language-server");
    press(&mut app, KeyCode::Left);
    assert_eq!(list_focus(&mut app), None);
    press_with(&mut app, KeyCode::Down, KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Esc);
    assert_eq!(list_focus(&mut app), None);
    assert!(
        matches!(mode(&app), Mode::Form(_)),
        "Esc closed only the list"
    );
    press_with(&mut app, KeyCode::Down, KeyModifiers::ALT);
    press(&mut app, KeyCode::Tab);
    assert_eq!(list_focus(&mut app), None);
    assert_eq!(focus(&mut app), FormField::Arguments);
}

#[test]
fn a_read_only_dialog_repeats_its_reason() {
    let machine = LspMachine::new();
    machine.write_settings("{broken");
    let mut app = machine.opened();
    for code in [
        KeyCode::Char(' '),
        KeyCode::Enter,
        KeyCode::F(4),
        KeyCode::Insert,
        KeyCode::F(7),
        KeyCode::Delete,
        KeyCode::F(8),
        KeyCode::F(2),
    ] {
        editor_mut(&mut app).set_mode(Mode::Browse);
        press(&mut app, code);
        assert!(
            matches!(editor(&app).notice(), Some(Notice::Error(message)) if message.starts_with("Read only: settings.yaml does not parse: ")),
            "{code:?}"
        );
        assert_eq!(mode(&app), &Mode::Browse, "{code:?}");
    }
    assert!(!editor(&app).dirty());
}

/// Typing a multibyte space into Extensions runs the suggestion refresh on
/// every key; it must neither panic (the release build aborts on a panic)
/// nor lose the word typed after it.
#[test]
fn a_multibyte_space_in_extensions_is_a_separator_not_a_crash() {
    for space in ['\u{a0}', '\u{3000}'] {
        let machine = LspMachine::new();
        let mut app = edit_python(&machine);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Tab);
        assert_eq!(focus(&mut app), FormField::Extensions);
        press(&mut app, KeyCode::End);
        press(&mut app, KeyCode::Char(space));
        type_text(&mut app, "rs");
        press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
        if list_focus(&mut app).is_some() {
            press(&mut app, KeyCode::Enter);
        }
        assert_eq!(
            field(&mut app, FormField::Extensions),
            format!(".py .pyi{space}.rs ")
        );
    }
}

/// The highlighted suggestion of an active list.
fn highlighted(app: &mut App) -> String {
    let form = editor_mut(app).form_mut().expect("form");
    let list = form.completion().expect("an open list");
    list.menu().selection().text.clone()
}

/// In an active list Down and Up move the highlight, and Shift+Tab leaves
/// the list for the previous control.
#[test]
fn an_active_list_takes_up_and_down_and_shift_tab_leaves_it() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    press(&mut app, KeyCode::Tab);
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    let first = highlighted(&mut app);
    press(&mut app, KeyCode::Down);
    assert_ne!(highlighted(&mut app), first);
    press(&mut app, KeyCode::Up);
    assert_eq!(highlighted(&mut app), first);

    press(&mut app, KeyCode::BackTab);
    assert_eq!(focus(&mut app), FormField::Command);
    assert_eq!(list_focus(&mut app), None);
}

/// Only an edit refreshes the suggestions: Delete at the end of the field
/// changes nothing and opens no list, where typing opens one.
#[test]
fn a_key_that_changes_nothing_opens_no_list() {
    let machine = LspMachine::new();
    let mut app = edit_python(&machine);
    let form = editor_mut(&mut app).form_mut().expect("form");
    form.focused_line_mut().expect("command").set_text("py");
    press(&mut app, KeyCode::Delete);
    assert_eq!(field(&mut app, FormField::Command), "py");
    assert_eq!(list_focus(&mut app), None);
    type_text(&mut app, "l");
    assert_eq!(list_focus(&mut app), Some(ListFocus::Field), "an edit does");
}
