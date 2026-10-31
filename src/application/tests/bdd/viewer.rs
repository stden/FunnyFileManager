//! F3 viewer: file content display, title, page-down scrolling, and close.

use super::{BehaviorWorld, Case};
use crate::models::Overlay;
use cucumber::{given, then};

/// Creates a file with the given content, reloads the left panel, and selects
/// the new entry so the next F3 / F4 key targets it.
#[given(expr = "a file named {string} containing {string}")]
fn file_with_content(world: &mut BehaviorWorld, name: String, content: String) {
    let case = world.case();
    std::fs::write(case.root.path().join("left").join(&name), &content).unwrap();
    case.app.left_panel.load_entries().unwrap();
    assert!(
        case.app.left_panel.select_named(&name),
        "file {name:?} must appear in the panel after load"
    );
    case.paint();
}

/// Creates a file whose lines read `"Line 001"`, `"Line 002"`, …, `"Line N"`.
/// The zero-padded format avoids substring collisions (`"Line 001"` does not
/// appear inside `"Line 010"` or `"Line 100"`).
#[given(expr = "a file named {string} with {int} numbered lines")]
fn file_with_numbered_lines(world: &mut BehaviorWorld, name: String, count: usize) {
    let content = (1..=count)
        .map(|i| format!("Line {:03}", i))
        .collect::<Vec<_>>()
        .join("\n");
    file_with_content(world, name, content);
}

#[then("the overlay is the viewer")]
fn is_viewer(world: &mut BehaviorWorld) {
    let overlay = &world.case().app.overlay;
    assert!(
        matches!(overlay, Overlay::Viewer(_)),
        "expected Viewer overlay, found {overlay:?}"
    );
}
