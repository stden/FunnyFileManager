//! Steps for panel selection marks, totals summary, and 8.3 short-name renumbering.

use super::BehaviorWorld;
use crate::models::ShortName;
use cucumber::{given, then, when};

#[given(expr = "a sized left file {string} of {int} bytes")]
fn sized_left_file(world: &mut BehaviorWorld, name: String, bytes: usize) {
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), vec![b'x'; bytes]).unwrap();
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[when("I clear the left panel marks and press Ctrl+M")]
fn clear_and_restore(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.left_panel.clear_marks();
    case.paint();
    assert_eq!(case.app.left_panel.marked.len(), 0);
    case.press("Ctrl+M");
}

#[then(expr = "the left panel has {int} marked entries")]
fn marked_count(world: &mut BehaviorWorld, expected: usize) {
    assert_eq!(world.case().app.left_panel.marked.len(), expected);
}

#[then(expr = "the left short name for {string} is {string}")]
fn short_name_is(world: &mut BehaviorWorld, name: String, expected: String) {
    let case = world.case();
    assert!(
        case.app.left_panel.options.short_names,
        "short names mode must be active"
    );
    let table = case.app.left_panel.short_names();
    assert_eq!(
        table.get(&name).map(ShortName::as_str),
        Some(expected.as_str())
    );
    let screen = case.screen.text();
    let stem = expected.split('.').next().unwrap_or(&expected);
    assert!(
        screen.contains(stem),
        "missing {stem:?} on screen\n{screen}"
    );
}
