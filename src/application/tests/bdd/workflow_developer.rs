//! Steps for the developer workflow BDD scenarios.
//!
//! Steps shared with other modules (workspace setup, key dispatch, overlay
//! assertions, editor open/type) are NOT redefined here — they live in
//! `common.rs`, `viewer.rs`, and `editor.rs`.

use super::BehaviorWorld;
use cucumber::{given, then};

/// The twenty canonical source-file names used by the developer workflow.
const SOURCE_FILES: &[&str] = &[
    "action.rs",
    "app.rs",
    "config.rs",
    "dispatch.rs",
    "editor.rs",
    "filter.rs",
    "handler.rs",
    "input.rs",
    "layout.rs",
    "logger.rs",
    "main.rs",
    "module_alpha.rs",
    "network.rs",
    "ops.rs",
    "panel.rs",
    "queue.rs",
    "render.rs",
    "settings.rs",
    "terminal.rs",
    "utils.rs",
];

#[given("the left panel has 20 source files with .rs extension")]
fn source_files(world: &mut BehaviorWorld) {
    let case = world.case();
    let left = case.root.path().join("left");
    for name in SOURCE_FILES {
        std::fs::write(left.join(name), b"// placeholder").unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
    // Ensure the cursor lands on a real file (not ".." parent-dir link)
    // so F3/F4 in subsequent steps open the viewer/editor rather than
    // navigating up.  "action.rs" is always present among the 20 files.
    case.app.left_panel.select_named("action.rs");
    case.paint();
}

#[then(expr = "the left panel cursor is on a file starting with {string}")]
fn cursor_on_file_starting_with(world: &mut BehaviorWorld, prefix: String) {
    let case = world.case();
    let entry = case.app.left_panel.get_selected_entry();
    assert!(
        entry.is_some_and(|e| e.name.starts_with(prefix.as_str())),
        "expected a file starting with {prefix:?}, got {:?}",
        entry.map(|e| e.name.as_str())
    );
}
