//! Purpose: the form's suggestion list, passive and active — how it opens, follows typing, takes keys, and closes.

use super::*;

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
