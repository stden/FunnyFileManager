//! Purpose: unit tests for the selection and the draft: moving, toggling,
//! Del, availability, and keeping the selection across a save.

use super::*;

#[test]
fn the_dialog_opens_on_the_first_server_and_clean() {
    let editor = editor(&json!({}));
    assert_eq!(selected(&editor), "bat");
    assert_eq!(editor.selected_index(), Some(0));
    assert!(!editor.dirty());
    assert_eq!(editor.mode(), &Mode::Browse);
    assert_eq!(editor.notice(), None);
    assert!(matches!(editor.access(), Access::Writable { .. }));
}

#[test]
fn the_selection_moves_within_the_list() {
    let mut editor = editor(&json!({}));
    let count = editor.draft().0.len();
    editor.move_selection(Step::Up);
    assert_eq!(editor.selected_index(), Some(0), "stops at the top");
    editor.move_selection(Step::Last);
    assert_eq!(editor.selected_index(), Some(count - 1));
    editor.move_selection(Step::Down);
    assert_eq!(
        editor.selected_index(),
        Some(count - 1),
        "stops at the bottom"
    );
    editor.record_drawn(Drawn::List { page: 4 });
    editor.move_selection(Step::PageUp);
    assert_eq!(editor.selected_index(), Some(count - 5));
    editor.move_selection(Step::PageDown);
    assert_eq!(editor.selected_index(), Some(count - 1));
    editor.move_selection(Step::First);
    assert_eq!(selected(&editor), "bat");
    editor.select(&language("python"));
    assert_eq!(selected(&editor), "python");
    editor.select(&language("not-there"));
    assert_eq!(
        selected(&editor),
        "python",
        "an unknown language is not selected"
    );
}

#[test]
fn a_toggled_server_is_changed_and_the_dialog_dirty() {
    let mut editor = editor(&json!({}));
    editor.select(&language("python"));
    assert_eq!(editor.toggle_selected(), Some(language("python")));
    assert!(editor.dirty());
    assert!(editor.is_changed(&language("python")));
    assert!(!editor.is_changed(&language("css")));
    assert_eq!(editor.draft().0["python"].enabled, Enablement::Disabled);
    editor.toggle_selected();
    assert!(!editor.dirty(), "back to what the file holds");
}

/// Del resets a catalog server's four owned fields and keeps the rest; on an
/// added server it removes it and selects the row now at its place.
#[test]
fn del_resets_a_catalog_server_and_removes_an_added_one() {
    let mut editor = editor(&json!({
        "python": {"command": "custom", "root_markers": ["setup.cfg"]},
        "mine": {"command": "mine-server", "extensions": ["mn"]}
    }));
    editor.select(&language("python"));
    assert_eq!(
        editor.delete_selected(),
        Some((language("python"), Origin::Catalog))
    );
    let python = &editor.draft().0["python"];
    assert_eq!(python.command, LspSettings::catalog().0["python"].command);
    assert_eq!(python.root_markers, ["setup.cfg"]);

    editor.select(&language("mine"));
    let index = editor.selected_index();
    assert_eq!(
        editor.delete_selected(),
        Some((language("mine"), Origin::Added))
    );
    assert!(!editor.draft().0.contains_key("mine"));
    assert_eq!(
        editor.selected_index(),
        index,
        "the next row takes its place"
    );
    assert_eq!(LspEditor::origin(&language("python")), Origin::Catalog);
    assert_eq!(LspEditor::origin(&language("mine")), Origin::Added);
}

/// A save brings in servers another program added or removed; the selection
/// stays on its language, and is clamped only when that language is gone.
#[test]
fn the_selection_follows_its_language_across_a_save() {
    let mut editor = editor(&json!({"zeta": {"command": "z"}}));
    editor.select(&language("python"));
    let before = editor.selected_index().expect("python");
    let with_earlier: LspSettings =
        serde_json::from_value(json!({"aaa": {"command": "a"}, "zeta": {"command": "z"}}))
            .expect("valid");
    editor.saved(with_earlier);
    assert_eq!(selected(&editor), "python");
    assert_eq!(editor.selected_index(), Some(before + 1));
    assert!(!editor.dirty());

    editor.move_selection(Step::Last);
    assert_eq!(selected(&editor), "zeta");
    let without: LspSettings = serde_json::from_value(json!({})).expect("valid");
    editor.saved(without);
    assert_eq!(selected(&editor), "yaml", "clamped to the new last row");
}

#[test]
fn availability_is_kept_per_language() {
    let mut editor = editor(&json!({}));
    let missing = Availability::NotInstalled(crate::lsp::NotInstalled::NoCache);
    editor.set_availability(language("python"), missing.clone());
    assert_eq!(editor.availability(&language("python")), Some(&missing));
    assert_eq!(editor.availability(&language("css")), None);
}

#[test]
fn the_accessors_show_the_selected_server_the_baseline_and_the_notice() {
    let mut editor = editor(&json!({}));
    editor.select(&language("python"));
    let (selected, server) = editor.selected_server().expect("python");
    assert_eq!(selected, &language("python"));
    assert_eq!(server, &editor.baseline().0["python"]);
    editor.set_notice(Notice::Info("Nothing to save".to_owned()));
    assert_eq!(
        editor.notice(),
        Some(&Notice::Info("Nothing to save".to_owned()))
    );
    *editor.mode_mut() = Mode::ConfirmExit(ExitChoice::Discard);
    assert_eq!(editor.mode(), &Mode::ConfirmExit(ExitChoice::Discard));
    assert!(editor.form_mut().is_none());
}

#[test]
fn an_empty_table_has_no_selection_to_move_or_toggle() {
    let mut editor = LspEditor::new(
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile),
        LspSettings(BTreeMap::new()),
    );
    assert_eq!(editor.selected(), None);
    editor.move_selection(Step::Down);
    assert_eq!(editor.selected_index(), None);
    assert_eq!(editor.toggle_selected(), None);
    assert_eq!(editor.delete_selected(), None);
    assert_eq!(editor.selected_server(), None);
}
