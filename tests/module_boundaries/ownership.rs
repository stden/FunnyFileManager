//! Purpose: Keep each operation's single owner, test placement and runtime lifecycle.

use super::architecture::{
    dependencies, functions, package_sources, test_attribute_lines, SourceFile,
};
use super::{repository, tui_sources};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[path = "ownership/abort_hook.rs"]
mod abort_hook;
#[path = "ownership/owners.rs"]
mod owners;
#[path = "ownership/test_code.rs"]
mod test_code;
use abort_hook::abort_hook_violations;
use owners::{owner_violations, present_files};
use test_code::{embedded_test_violations, feature_selection_violations};

/// The single source file at `relative` in `sources`.
#[expect(
    clippy::panic,
    reason = "test support: a file missing from the source set fails the test that named it"
)]
fn source<'s>(sources: &'s [SourceFile], relative: &str) -> &'s SourceFile {
    sources
        .iter()
        .find(|file| file.relative == relative)
        .unwrap_or_else(|| panic!("{relative} is not in the source set"))
}

/// Implementation files the compatibility facade once owned.
const OBSOLETE_HANDLER_FILES: [&str; 5] = [
    "src/handlers/dispatch.rs",
    "src/handlers/dispatch/tests.rs",
    "src/handlers/file_ops.rs",
    "src/handlers/mcp.rs",
    "src/handlers/shell.rs",
];

const CANONICAL_OWNERS: [(&str, &str); 11] = [
    ("execute_command", "src/ops/dispatch.rs"),
    ("handle_copy", "src/ops/file/transfer.rs"),
    ("handle_move", "src/ops/file/transfer.rs"),
    ("handle_mkdir", "src/ops/file/create.rs"),
    ("handle_delete_confirm", "src/ops/file/delete.rs"),
    ("handle_delete", "src/ops/file/delete.rs"),
    ("handle_create_file", "src/ops/file/create.rs"),
    ("rename_or_fail", "src/ops/file.rs"),
    ("run_shell_command", "src/ops/shell.rs"),
    ("handle_view", "src/ops/viewer.rs"),
    ("open_mcp_status", "src/ops/viewer.rs"),
];

#[test]
fn operations_have_one_canonical_owner() {
    assert_eq!(
        owner_violations(&tui_sources(), &CANONICAL_OWNERS),
        Vec::<String>::new()
    );
}

#[test]
fn unit_test_bodies_live_in_separate_files() {
    let sources = ["", "mcp", "colorer-sys"]
        .iter()
        .flat_map(|package| package_sources(repository(), package))
        .collect::<Vec<_>>();
    assert_eq!(embedded_test_violations(&sources), Vec::<String>::new());
}

#[test]
fn test_helpers_do_not_select_runtime_implementations() {
    let sources = tui_sources();
    for (relative, name) in [
        ("src/bootstrap.rs", "run"),
        ("src/bootstrap.rs", "main_impl"),
        ("src/editor.rs", "handle_edit"),
    ] {
        let violations = feature_selection_violations(source(&sources, relative), name);
        assert_eq!(violations, Vec::<String>::new());
    }
}

#[test]
fn abort_hook_precedes_terminal_activation() {
    let sources = tui_sources();
    let violations = abort_hook_violations(source(&sources, "src/bootstrap.rs"));
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn obsolete_handler_implementation_files_are_removed() {
    assert_eq!(
        present_files(repository(), &OBSOLETE_HANDLER_FILES),
        Vec::<String>::new()
    );
}
