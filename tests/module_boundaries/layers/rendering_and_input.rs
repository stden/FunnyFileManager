//! Purpose: Rendering and key resolution never perform operations, I/O or OS calls.

use super::*;

pub fn ui_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("ui");
    messages(&forbidden_dependencies(
        sources,
        checked,
        &RENDERING_FORBIDDEN,
    ))
}

pub fn action_violations(sources: &[SourceFile]) -> Vec<String> {
    let forbidden = [RENDERING_FORBIDDEN.as_slice(), &INPUT_ONLY_FORBIDDEN].concat();
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("action");
    messages(&forbidden_dependencies(sources, checked, &forbidden))
}

pub fn keymap_violations(sources: &[SourceFile]) -> Vec<String> {
    let forbidden = [
        RENDERING_FORBIDDEN.as_slice(),
        &INPUT_ONLY_FORBIDDEN,
        &KEYMAP_ONLY_FORBIDDEN,
    ]
    .concat();
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("keymap");
    messages(&forbidden_dependencies(sources, checked, &forbidden))
}

#[test]
fn ui_rule_reports_operations_and_environment_mutation() {
    let sources = sources(&[
        (
            "src/ui/status.rs",
            "use crate::{models::App, ops::file};\n\nfn draw() {\n    std::env::set_var(\"TERM\", \"x\");\n}\n",
        ),
        ("src/ui/tests.rs", "use crate::ops;\n"),
        ("src/models/app.rs", "use crate::ops;\n"),
    ]);
    assert_eq!(
        ui_violations(&sources),
        [
            "src/ui/status.rs:1: `crate::ops::file` (forbidden: `crate::ops`)",
            "src/ui/status.rs:4: `std::env::set_var` (forbidden: `std::env::set_var`)",
        ]
    );
}

#[test]
fn ui_rule_follows_crate_root_reexports() {
    let sources = sources(&[
        (
            "src/lib.rs",
            "mod application;\npub mod ui;\npub use application::run_app;\n",
        ),
        (
            "src/ui/status.rs",
            "fn restart() {\n    crate::run_app();\n}\n",
        ),
    ]);
    assert_eq!(
        ui_violations(&sources),
        ["src/ui/status.rs:2: `crate::application::run_app` (forbidden: `crate::application`)"]
    );
}

#[test]
fn action_rule_also_rejects_io_and_environment_reads() {
    let reads_input = "use std::io::Read;\n\nfn shell() -> Option<String> {\n    std::env::var(\"SHELL\").ok()\n}\n";
    let sources = sources(&[
        ("src/action/keys.rs", reads_input),
        ("src/ui/title.rs", reads_input),
    ]);
    assert_eq!(
        action_violations(&sources),
        [
            "src/action/keys.rs:1: `std::io::Read` (forbidden: `std::io`)",
            "src/action/keys.rs:4: `std::env::var` (forbidden: `std::env`)",
        ]
    );
    assert_eq!(ui_violations(&sources), Vec::<String>::new());
}

/// Rendering and key resolution never reach the OS adapters: a renderer that
/// read `/proc` or signalled a process would bypass the models it draws.
#[test]
fn ui_and_action_rules_reject_the_os_adapters() {
    let reads_processes = "use crate::process_table::SystemHost;\n\nfn ticks() -> u64 {\n    crate::platform::clock_ticks_per_second()\n}\n";
    let sources = sources(&[
        ("src/ui/tasks.rs", reads_processes),
        ("src/action/tasks.rs", reads_processes),
    ]);
    assert_eq!(
        ui_violations(&sources),
        [
            "src/ui/tasks.rs:1: `crate::process_table::SystemHost` (forbidden: `crate::process_table`)",
            "src/ui/tasks.rs:4: `crate::platform::clock_ticks_per_second` (forbidden: `crate::platform`)",
        ]
    );
    assert_eq!(
        action_violations(&sources),
        [
            "src/action/tasks.rs:1: `crate::process_table::SystemHost` (forbidden: `crate::process_table`)",
            "src/action/tasks.rs:4: `crate::platform::clock_ticks_per_second` (forbidden: `crate::platform`)",
        ]
    );
}

/// The keymap names commands and chords, nothing that acts: a table that
/// read `ffm.yml` itself, or drew a label, would be a second owner of both.
#[test]
fn keymap_rule_rejects_io_rendering_and_configuration() {
    let reads_the_file = "use crate::config::dir;\nuse crate::ui::palette;\n\nfn text() -> std::io::Result<String> {\n    std::fs::read_to_string(crate::ops::keymap::path())\n}\n";
    let sources = sources(&[
        ("src/keymap/load.rs", reads_the_file),
        ("src/keymap/tests.rs", reads_the_file),
        (
            "src/keymap.rs",
            "use crate::action::{Action, Bindings};\nuse crate::models::PanelSide;\n",
        ),
    ]);
    assert_eq!(
        keymap_violations(&sources),
        [
            "src/keymap/load.rs:1: `crate::config::dir` (forbidden: `crate::config`)",
            "src/keymap/load.rs:2: `crate::ui::palette` (forbidden: `crate::ui`)",
            "src/keymap/load.rs:4: `std::io::Result` (forbidden: `std::io`)",
            "src/keymap/load.rs:5: `std::fs::read_to_string` (forbidden: `std::fs`)",
            "src/keymap/load.rs:5: `crate::ops::keymap::path` (forbidden: `crate::ops`)",
        ]
    );
    assert_eq!(action_violations(&sources), Vec::<String>::new());
}
