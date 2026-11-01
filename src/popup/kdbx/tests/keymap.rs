//! Purpose: the vault reads its keys from the keymap — a default key with a modifier more does nothing, and a key the keymap gives an action runs it in the list, the entry form and the password dialog.

use super::*;
use crate::action::vault::VaultCommand;
use crate::kdbx::vault::{BrowsePage, Page};
use crate::keymap::{tables, ActionName, Explicit};
use crate::test_support::chord;

// TODO(Rule of Three): `popup/kdbx/tests/errors.rs` has the same helper; a
// third user moves both to `test_support`.
fn vault(app: &App) -> &crate::kdbx::Vault {
    app.kdbx_session()
        .and_then(KdbxSession::vault)
        .expect("unlocked vault")
}

fn form_title(app: &App) -> String {
    vault(app).form().expect("form").title().to_owned()
}

fn press(app: &mut App, text: &str) -> bool {
    process_popup_input(app, chord(text).to_event())
}

/// Gives the vault the keys `ffm.yml` would: each action with its one key.
// TODO(Rule of Three): `action::vault::tests::rebound` builds the same
// bindings for one action; a third user moves both to `test_support`.
fn rebind(app: &mut App, keys: &[(&'static str, &str)]) {
    let explicit: Explicit = keys
        .iter()
        .map(|&(action, text)| (ActionName::new(action), vec![chord(text)]))
        .collect();
    app.keymap.vault = tables::VAULT.layered(&explicit).bindings;
}

#[test]
fn a_default_key_with_a_modifier_more_is_swallowed_and_does_nothing() {
    let (_dir, mut app) = app_with_unlocked_vault();
    focus(&mut app, "GitHub");
    let before = vault(&app).selected();

    for text in [
        "Shift+Down",
        "Ctrl+End",
        "Shift+F4",
        "Alt+F6",
        "Shift+F8",
        "Alt+Enter",
        "Ctrl+Shift+R",
        "Ctrl+Alt+Z",
        "Shift+Backspace",
        "Shift+Esc",
    ] {
        assert!(press(&mut app, text), "{text} reached the panels");
        let vault = vault(&app);
        assert_eq!(vault.page, Page::Browse(BrowsePage::Tree), "{text}");
        assert_eq!(vault.selected(), before, "{text}");
        assert_eq!(vault.filter(), "GitHub", "{text}");
        assert!(vault.form().is_none(), "{text}");
        assert!(!vault.is_revealed(), "{text}");
    }
}

#[test]
fn a_key_the_keymap_gives_an_action_runs_it_and_its_default_key_does_not() {
    let (_dir, mut app) = app_with_unlocked_vault();
    rebind(&mut app, &[("toggle_view", "Alt+V")]);

    press(&mut app, "F6");
    assert_eq!(vault(&app).page, Page::Browse(BrowsePage::Tree));

    press(&mut app, "Alt+V");
    assert_eq!(vault(&app).page, Page::Browse(BrowsePage::Entries));
}

#[test]
fn the_password_dialog_closes_on_the_key_that_reveals() {
    let (_dir, mut app) = app_with_unlocked_vault();
    rebind(&mut app, &[("reveal", "F3")]);
    focus(&mut app, "GitHub");

    press(&mut app, "F3");
    assert!(matches!(vault(&app).page, Page::Password(_)));

    press(&mut app, "Ctrl+R");
    assert!(matches!(vault(&app).page, Page::Password(_)));

    press(&mut app, "F3");
    assert!(matches!(vault(&app).page, Page::Browse(_)));
    assert!(!vault(&app).is_revealed());
}

#[test]
fn the_entry_form_saves_and_undoes_on_the_keys_the_keymap_gives() {
    let (_dir, mut app) = app_with_unlocked_vault();
    rebind(&mut app, &[("undo", "Alt+Z"), ("save", "Ctrl+S")]);
    focus(&mut app, "GitHub");
    press(&mut app, "F4");
    type_str(&mut app, "XY");

    press(&mut app, "Ctrl+Z");
    press(&mut app, "F2");
    assert_eq!(form_title(&app), "GitHubXY");

    press(&mut app, "Alt+Z");
    assert_eq!(form_title(&app), "GitHubX");

    press(&mut app, "Ctrl+S");
    assert!(vault(&app).form().is_none());
    assert!(vault(&app).visible().any(|entry| entry.title == "GitHubX"));
}

/// The form takes `Save` and `Undo` from the vault's keys and nothing
/// else: a character bound to another action is still text there.
#[test]
fn a_character_bound_to_a_list_command_is_typed_in_the_entry_form_alone() {
    let (_dir, mut app) = app_with_unlocked_vault();
    rebind(&mut app, &[("toggle_view", "x")]);
    assert_eq!(
        app.keymap.vault.command(chord("x")),
        Some(VaultCommand::ToggleView)
    );
    focus(&mut app, "GitHub");

    press(&mut app, "x");
    assert_eq!(vault(&app).page, Page::Browse(BrowsePage::Entries));
    assert_eq!(vault(&app).filter(), "GitHub", "`x` was typed");

    press(&mut app, "x");
    press(&mut app, "F4");
    press(&mut app, "x");
    assert_eq!(form_title(&app), "GitHubx");
}
