//! Steps for Alt+F7 file search by name mask and case-insensitive text content.

use super::BehaviorWorld;
use crate::models::{MenuDialog, Overlay};
use cucumber::{given, then};

#[given(expr = "a nested left file {string} containing {string}")]
fn nested_left_file(world: &mut BehaviorWorld, relative: String, content: String) {
    write_nested(world, &relative, content.as_bytes());
}

#[given(expr = "a binary left file {string} containing NUL and {string}")]
fn binary_left_file(world: &mut BehaviorWorld, relative: String, tail: String) {
    let mut bytes = b"BIN\0".to_vec();
    bytes.extend_from_slice(tail.as_bytes());
    write_nested(world, &relative, &bytes);
}

fn write_nested(world: &mut BehaviorWorld, relative: &str, bytes: &[u8]) {
    let case = world.case();
    let full = case.root.path().join("left").join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, bytes).unwrap();
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[then(expr = "the search results dialog lists {int} match")]
fn search_result_count(world: &mut BehaviorWorld, expected: usize) {
    let Overlay::MenuDialog(dialog) = &world.case().app.overlay else {
        panic!("expected search results MenuDialog");
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("expected Choices dialog");
    };
    assert_eq!(rows.len(), expected);
}

#[then("the viewer overlay is open")]
fn viewer_overlay_open(world: &mut BehaviorWorld) {
    assert!(
        matches!(world.case().app.overlay, Overlay::Viewer(_)),
        "expected Viewer overlay"
    );
}
