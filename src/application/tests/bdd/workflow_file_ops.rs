//! Steps for the file-operations workflow BDD scenarios.
//!
//! "the left panel contains the file {string}" is defined in `quick_search.rs`
//! and is not redefined here. Steps shared with other modules (workspace
//! setup, key dispatch, screen assertions, overlay closed) live in `common.rs`.

use super::{BehaviorWorld, Case};
use crate::models::{NamePurpose, Overlay};
use cucumber::{given, then};

/// Move the left-panel cursor onto the named file.
#[given(expr = "{string} is selected in the left panel")]
fn select_file(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    let found = case.app.left_panel.select_named(&name);
    assert!(found, "{name:?} must be in the left panel");
    case.paint();
}

#[given(expr = "the {word} file {string} contains {string}")]
fn side_file_contains(world: &mut BehaviorWorld, side: String, name: String, content: String) {
    let case = world.case();
    std::fs::write(case.root.path().join(&side).join(&name), content).unwrap();
    case.app
        .panel_mut(Case::side(&side))
        .load_entries()
        .unwrap();
    case.paint();
}

#[then(expr = "the rename prompt targets {string}")]
fn rename_prompt(world: &mut BehaviorWorld, name: String) {
    let Overlay::NamePrompt(prompt) = &world.case().app.overlay else {
        panic!("Shift+F6 must open the rename prompt");
    };
    assert_eq!(prompt.purpose(), NamePurpose::Rename);
    assert_eq!(prompt.original(), name);
    assert_eq!(prompt.name(), name);
}

#[then(expr = "the {word} panel directory {string} exists on disk")]
fn directory_exists(world: &mut BehaviorWorld, side: String, name: String) {
    let path = world.case().root.path().join(&side).join(&name);
    assert!(path.is_dir(), "expected directory {}", path.display());
}

#[then(expr = "the {word} file {string} equals {string}")]
fn side_file_equals(world: &mut BehaviorWorld, side: String, name: String, expected: String) {
    let path = world.case().root.path().join(&side).join(&name);
    let actual = std::fs::read_to_string(&path).unwrap();
    assert_eq!(actual, expected);
}

#[then(expr = "the {word} file {string} is absent on disk")]
fn side_file_absent(world: &mut BehaviorWorld, side: String, name: String) {
    let path = world.case().root.path().join(&side).join(&name);
    assert!(!path.exists(), "expected {} to be absent", path.display());
}
