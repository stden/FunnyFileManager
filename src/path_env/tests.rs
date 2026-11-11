//! Purpose: Unit tests for `PathEditor` splitting, dirty tracking, removal, and reordering.

use super::*;

fn snap(value: &str) -> Snapshot {
    Snapshot {
        value: value.to_owned(),
        revision: "rev-1".to_owned(),
        location: "User PATH".to_owned(),
    }
}

#[test]
fn scope_and_validation_labels_cover_every_variant() {
    assert_eq!(Scope::User.label(), "current user");
    assert_eq!(Scope::System.label(), "all users");
    assert_eq!(Validation::Directory.label(), "OK");
    assert_eq!(Validation::Missing.label(), "MISSING");
    assert_eq!(Validation::NotDirectory.label(), "NOT A DIRECTORY");
    assert_eq!(Validation::Unresolved.label(), "UNKNOWN VARIABLE");
    assert_eq!(Validation::Relative.label(), "NOT ABSOLUTE");
    assert_eq!(Validation::Empty.label(), "EMPTY");
}

#[test]
fn empty_and_trailing_separator_snapshots_preserve_dirty_and_empty_removal_invariants() {
    let mut empty = PathEditor::new(Scope::User, snap(""));
    assert!(empty.entries.is_empty());
    assert!(!empty.dirty());
    empty.remove();
    empty.move_selection(true, true);
    empty.move_selection(false, true);
    assert_eq!(empty.selected, 0);

    let sep = separator();
    let raw = format!("alpha{sep}");
    let editor = PathEditor::new(Scope::System, snap(&raw));
    assert_eq!(editor.entries.len(), 2);
    assert_eq!(editor.entries[1].text, "");
    assert!(!editor.dirty());
}

#[test]
fn selection_movement_reordering_edit_focus_and_removal_clamp_safely() {
    let sep = separator();
    let raw = format!("first{sep}second{sep}third");
    let mut editor = PathEditor::new(Scope::User, snap(&raw));
    assert!(editor.focused_line_mut().is_none());

    editor.move_selection(false, true);
    assert_eq!(editor.selected, 0);
    editor.move_selection(true, true);
    assert_eq!(editor.selected, 1);
    assert!(editor.dirty());
    assert_eq!(editor.value(), format!("second{sep}first{sep}third"));

    editor.edit(Some(1));
    editor.focused_line_mut().unwrap().insert_char('!');
    editor.mode = Mode::ConfirmSave;
    assert!(editor.focused_line_mut().is_none());

    editor.selected = 2;
    editor.remove();
    assert_eq!(editor.selected, 1);
    editor.remove();
    editor.remove();
    assert!(editor.entries.is_empty());
    assert_eq!(editor.selected, 0);
}
