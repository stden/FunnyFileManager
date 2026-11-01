//! Alt-letter quick search: panel cursor jumps to the first matching entry.

use super::BehaviorWorld;
use cucumber::{given, then};

/// Adds a file to the left panel without selecting it.  Calling this step
/// multiple times in a Background block populates the panel with several
/// entries for the quick-search scenarios to filter.
#[given(expr = "the left panel contains the file {string}")]
fn add_file(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), "").unwrap();
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[then("quick search is active")]
fn active(world: &mut BehaviorWorld) {
    assert!(world.case().app.quick_search.is_active());
}

#[then("quick search is inactive")]
fn inactive(world: &mut BehaviorWorld) {
    assert!(!world.case().app.quick_search.is_active());
}
