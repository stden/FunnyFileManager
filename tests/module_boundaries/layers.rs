//! Purpose: Keep rendering, key resolution, state and the facades in their layers.

use super::architecture::{
    dependencies, facade_violations, forbidden_dependencies, starts_with_path, SourceFile,
};
use super::{messages, tui_sources};

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

fn ui_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("ui");
    messages(&forbidden_dependencies(
        sources,
        checked,
        &RENDERING_FORBIDDEN,
    ))
}

fn action_violations(sources: &[SourceFile]) -> Vec<String> {
    let forbidden = [RENDERING_FORBIDDEN.as_slice(), &INPUT_ONLY_FORBIDDEN].concat();
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("action");
    messages(&forbidden_dependencies(sources, checked, &forbidden))
}

fn models_violations(sources: &[SourceFile]) -> Vec<String> {
    let checked = |file: &SourceFile| !file.is_test() && file.in_module("models");
    let mut violations = forbidden_dependencies(sources, checked, &MODELS_FORBIDDEN);
    violations.retain(|violation| !MODELS_KNOWN_INVERSIONS.contains(&violation.path.as_str()));
    messages(&violations)
}

/// The crate root and `handlers` only declare modules and re-exports.
fn facade_rule_violations(sources: &[SourceFile]) -> Vec<String> {
    sources
        .iter()
        .filter(|file| file.relative == "src/lib.rs" || file.in_module("handlers"))
        .flat_map(facade_violations)
        .collect()
}

/// `handlers` only forwards `crate::ops` and `crate::popup`.
fn handlers_forwarding_violations(sources: &[SourceFile]) -> Vec<String> {
    let mut messages = Vec::new();
    for file in sources.iter().filter(|file| file.in_module("handlers")) {
        let foreign = dependencies(file).into_iter().filter(|dependency| {
            !["crate::ops", "crate::popup"]
                .iter()
                .any(|target| starts_with_path(&dependency.path, target))
        });
        messages.extend(foreign.map(|dependency| {
            format!(
                "{}:{}: facade re-exports `{}`; it only forwards `crate::ops` and `crate::popup`",
                file.relative, dependency.line, dependency.path
            )
        }));
    }
    messages
}

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

#[test]
fn models_rule_reports_upward_dependencies() {
    let sources = sources(&[
        (
            "src/models/panel.rs",
            "use super::super::ui::panel_page_size;\n",
        ),
        (
            "src/models/editor.rs",
            "use crate::models::editor::Cursor;\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/panel.rs:1: `crate::ui::panel_page_size` (forbidden: `crate::ui`)"]
    );
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

/// The models define the process ports and the process table fills them, so
/// the models never name it; their one existing `platform` use stays legal.
#[test]
fn models_rule_rejects_the_process_table_but_not_platform() {
    let sources = sources(&[
        ("src/models/tasks.rs", "use crate::process_table::ProcFs;\n"),
        (
            "src/models/drive_entry.rs",
            "use crate::platform::DriveKind;\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/tasks.rs:1: `crate::process_table::ProcFs` (forbidden: `crate::process_table`)"]
    );
}

#[test]
fn models_rule_tolerates_only_the_reported_inversion() {
    let sources = sources(&[
        ("src/lib.rs", "mod application;\npub use application::{run_app, CLIPBOARD_CLEAR};\n"),
        (
            "src/models/clipboard.rs",
            "fn delay() -> Duration {\n    crate::CLIPBOARD_CLEAR\n}\n\nfn restart() {\n    crate::run_app();\n}\n",
        ),
    ]);
    assert_eq!(
        models_violations(&sources),
        ["src/models/clipboard.rs:6: `crate::application::run_app` (forbidden: `crate::application`)"]
    );
}

#[test]
fn facade_rule_reports_behaviour_test_code_and_foreign_reexports() {
    let sources = sources(&[
        ("src/lib.rs", "pub mod handlers;\n\npub fn run() {}\n"),
        (
            "src/handlers/mod.rs",
            "pub use crate::ops::handle_copy;\npub use crate::models::App;\n\n#[cfg(test)]\nmod tests;\n\npub mod shell {\n    pub fn run() {}\n}\n",
        ),
    ]);
    assert_eq!(
        facade_rule_violations(&sources),
        [
            "src/lib.rs:3: facade owns an item other than `mod` or `use`",
            "src/handlers/mod.rs:4: facade carries `#[cfg(test)]` code",
            "src/handlers/mod.rs:8: facade owns an item other than `mod` or `use`",
        ]
    );
    assert_eq!(
        handlers_forwarding_violations(&sources),
        ["src/handlers/mod.rs:2: facade re-exports `crate::models::App`; it only forwards `crate::ops` and `crate::popup`"]
    );
}
