//! High-level domain post-condition and invariant checks for FFM executable specifications.

use cucumber::then;

use super::{
    spec_dsl::{panel_entry_path, parse_side},
    BehaviorWorld,
};
use crate::{application::automation::current_context, keymap::Context, models::Overlay};

#[then(expr = "focus is on the {word} panel")]
pub(super) fn then_focus_is_on(world: &mut BehaviorWorld, side: String) {
    assert_eq!(world.case().app.active_panel, parse_side(&side));
}

#[then(expr = "the cursor in the {word} panel rests on {string}")]
pub(super) fn then_cursor_rests_on(world: &mut BehaviorWorld, side: String, expected: String) {
    let panel = world.case().app.panel(parse_side(&side));
    let selected = panel.get_selected_entry().map(|entry| entry.name.as_str());
    assert_eq!(selected, Some(expected.as_str()));
}

#[then(expr = "in the {word} panel file {string} has content {string}")]
pub(super) fn then_panel_file_content(
    world: &mut BehaviorWorld,
    side: String,
    name: String,
    expected: String,
) {
    let panel_side = parse_side(&side);
    let path = panel_entry_path(world.case(), panel_side, &name);
    let actual = std::fs::read_to_string(&path).unwrap();
    assert_eq!(actual, expected.replace("\\n", "\n"));
}

#[then(expr = "in the {word} panel entry {string} is absent")]
pub(super) fn then_panel_entry_absent(world: &mut BehaviorWorld, side: String, name: String) {
    let panel_side = parse_side(&side);
    let case = world.case();
    let path = panel_entry_path(case, panel_side, &name);
    assert!(
        !path.exists(),
        "expected {} to be absent on disk",
        path.display()
    );
    assert!(
        !case
            .app
            .panel(panel_side)
            .entries
            .iter()
            .any(|e| e.name == name),
        "expected {name:?} to be absent from {side} panel listing"
    );
}

#[then(expr = "in the {word} panel folder {string} exists")]
pub(super) fn then_panel_folder_exists(world: &mut BehaviorWorld, side: String, name: String) {
    let panel_side = parse_side(&side);
    let path = panel_entry_path(world.case(), panel_side, &name);
    assert!(path.is_dir(), "expected folder {} to exist", path.display());
}

#[then("the operation is rejected because the target already exists")]
pub(super) fn then_rejected_already_exists(world: &mut BehaviorWorld) {
    let case = world.case();
    assert!(matches!(case.app.overlay, Overlay::None));
    let screen = case.screen.text();
    assert!(
        screen.contains("already exists"),
        "expected conflict message on screen:\n{screen}"
    );
}

#[then(expr = "the active domain context is {string}")]
pub(super) fn then_active_domain_context(world: &mut BehaviorWorld, expected: String) {
    let canonical = match expected.as_str() {
        "Viewer" | "Просмотрщик" => "viewer",
        "Editor" | "Редактор" => "editor",
        _ => "panels",
    };
    let actual = current_context(&world.case().app).map(Context::name);
    assert_eq!(actual, Some(canonical));
}

#[then(expr = "the {word} panel has marked entries {string}")]
pub(super) fn then_marked_entries_list(world: &mut BehaviorWorld, side: String, expected: String) {
    let panel = world.case().app.panel(parse_side(&side));
    let mut actual: Vec<&str> = panel
        .marked
        .iter()
        .map(crate::models::FileName::as_str)
        .collect();
    actual.sort_unstable();
    let mut wanted: Vec<&str> = expected
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    wanted.sort_unstable();
    assert_eq!(actual, wanted);
}

#[then(expr = "the {word} panel entries are ordered as {string}")]
pub(super) fn then_panel_entries_ordered(
    world: &mut BehaviorWorld,
    side: String,
    expected: String,
) {
    let panel = world.case().app.panel(parse_side(&side));
    let actual: Vec<&str> = panel
        .entries
        .iter()
        .filter(|e| e.name != "..")
        .map(|e| e.name.as_str())
        .collect();
    let wanted: Vec<&str> = expected.split(',').map(str::trim).collect();
    assert_eq!(actual, wanted);
}
