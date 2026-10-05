//! Purpose: Keep rendering, key resolution, state and the facades in their layers.

use super::architecture::{
    dependencies, facade_violations, forbidden_dependencies, starts_with_path, SourceFile,
};
use super::{messages, tui_sources};

#[path = "layers/facade.rs"]
mod facade;
#[path = "layers/models.rs"]
mod models;
#[path = "layers/rendering_and_input.rs"]
mod rendering_and_input;
use facade::{facade_rule_violations, handlers_forwarding_violations};
use models::models_violations;
use rendering_and_input::{action_violations, ui_violations};

/// Effects rendering never performs: storage, processes, network, process
/// environment, every module that coordinates or executes operations, and the
/// OS adapters (`platform`, and the process table built on it) — rendering
/// draws the process list the models hold, it never reads one.
pub const RENDERING_FORBIDDEN: [&str; 17] = [
    "std::fs",
    "std::process",
    "std::net",
    "std::env::set_var",
    "std::env::remove_var",
    "std::env::set_current_dir",
    "crate::application",
    "crate::bootstrap",
    "crate::directory",
    "crate::diskfree",
    "crate::ops",
    "crate::handlers",
    "crate::popup",
    "crate::editor",
    "crate::terminal",
    "crate::process_table",
    "crate::platform",
];

/// Key resolution is a pure mapping: it does not read the environment or do I/O either.
const INPUT_ONLY_FORBIDDEN: [&str; 2] = ["std::io", "std::env"];

/// Layers above state, and the effects state never performs. The process
/// table is an adapter above the models: they define its ports, it fills them.
const MODELS_FORBIDDEN: [&str; 14] = [
    "crate::ui",
    "crate::action",
    "crate::popup",
    "crate::application",
    "crate::bootstrap",
    "crate::handlers",
    "crate::ops",
    "crate::terminal",
    "crate::editor",
    "std::process",
    "std::net",
    "std::env::set_var",
    "std::env::remove_var",
    "crate::process_table",
];

/// Existing inversion, reported with the hardening work rather than fixed here:
/// the clipboard auto-clear delay is owned by `application` (re-exported at the
/// crate root) and read by `models::clipboard`.
const MODELS_KNOWN_INVERSIONS: [&str; 1] = ["crate::application::CLIPBOARD_CLEAR"];

#[test]
fn ui_renders_without_executing_operations() {
    assert_eq!(ui_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn action_resolves_input_without_io() {
    assert_eq!(action_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn models_do_not_reach_into_higher_layers() {
    assert_eq!(models_violations(&tui_sources()), Vec::<String>::new());
}

#[test]
fn crate_root_and_compatibility_facade_only_wire_modules() {
    let sources = tui_sources();
    assert_eq!(facade_rule_violations(&sources), Vec::<String>::new());
    assert_eq!(
        handlers_forwarding_violations(&sources),
        Vec::<String>::new()
    );
}

fn sources(files: &[(&str, &str)]) -> Vec<SourceFile> {
    files
        .iter()
        .map(|(relative, text)| SourceFile::new(relative, text))
        .collect()
}
