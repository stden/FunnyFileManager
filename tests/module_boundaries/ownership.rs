//! Purpose: Keep each operation's single owner, test placement and runtime lifecycle.

use super::architecture::{
    dependencies, functions, package_sources, test_attribute_lines, SourceFile,
};
use super::{repository, tui_sources};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// The single source file at `relative` in `sources`.
fn source<'s>(sources: &'s [SourceFile], relative: &str) -> &'s SourceFile {
    sources
        .iter()
        .find(|file| file.relative == relative)
        .unwrap_or_else(|| panic!("{relative} is not in the source set"))
}

/// Files that define an exported (`pub` or `pub(crate)`) function, by name.
fn exported_function_owners(sources: &[SourceFile]) -> BTreeMap<String, Vec<String>> {
    let mut owners = BTreeMap::<String, Vec<String>>::new();
    for file in sources {
        for function in functions(file)
            .into_iter()
            .filter(|function| function.exported)
        {
            let files = owners.entry(function.name).or_default();
            if !files.contains(&file.relative) {
                files.push(file.relative.clone());
            }
        }
    }
    owners
}

/// Each operation is defined, exported, in exactly one canonical file.
fn owner_violations(sources: &[SourceFile], canonical: &[(&str, &str)]) -> Vec<String> {
    let owners = exported_function_owners(sources);
    canonical
        .iter()
        .filter_map(|(symbol, owner)| {
            let found = owners.get(*symbol).cloned().unwrap_or_default();
            (found != [*owner])
                .then(|| format!("`{symbol}` is defined in {found:?}; only in {owner}"))
        })
        .collect()
}

/// Test bodies live in `cfg(test)` child files, never in production sources.
fn embedded_test_violations(sources: &[SourceFile]) -> Vec<String> {
    sources
        .iter()
        .filter(|file| !file.is_test())
        .flat_map(|file| {
            test_attribute_lines(file).into_iter().map(|line| {
                format!(
                    "{}:{line}: test attribute in production code; use a cfg(test) child file",
                    file.relative
                )
            })
        })
        .collect()
}

/// `name` is defined once in `file`, and never chosen by `feature = "test-helpers"`.
fn feature_selection_violations(file: &SourceFile, name: &str) -> Vec<String> {
    let definitions = functions(file)
        .into_iter()
        .filter(|function| function.name == name)
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        let count = definitions.len();
        return vec![format!(
            "{}: `{name}` has {count} definitions; expected one",
            file.relative
        )];
    }
    definitions
        .iter()
        .filter(|function| function.selects_test_helpers)
        .map(|function| {
            let line = function.lines.start();
            format!(
                "{}:{line}: `{name}` is selected by the `test-helpers` feature",
                file.relative
            )
        })
        .collect()
}

/// The panic hook that restores the terminal is installed before the terminal changes.
fn abort_hook_violations(bootstrap: &SourceFile) -> Vec<String> {
    let Some(session) = functions(bootstrap)
        .into_iter()
        .find(|function| function.name == "run_session")
    else {
        return vec![format!("{}: no `run_session`", bootstrap.relative)];
    };
    let calls = dependencies(bootstrap)
        .into_iter()
        .filter(|dependency| session.lines.contains(&dependency.line))
        .collect::<Vec<_>>();
    let first = |path: &str| {
        calls
            .iter()
            .find(|dependency| dependency.path == path)
            .map(|dependency| (dependency.line, dependency.column))
    };
    let hook = first("std::panic::set_hook");
    let enter = first("crate::terminal::TerminalSession::enter");
    if matches!((hook, enter), (Some(hook), Some(enter)) if hook < enter) {
        return Vec::new();
    }
    vec![format!(
        "{}: `run_session` must call `std::panic::set_hook` (at {hook:?}) before \
         `TerminalSession::enter` (at {enter:?})",
        bootstrap.relative
    )]
}

/// Implementation files the compatibility facade once owned.
const OBSOLETE_HANDLER_FILES: [&str; 5] = [
    "src/handlers/dispatch.rs",
    "src/handlers/dispatch/tests.rs",
    "src/handlers/file_ops.rs",
    "src/handlers/mcp.rs",
    "src/handlers/shell.rs",
];

fn present_files(root: &Path, relatives: &[&str]) -> Vec<String> {
    relatives
        .iter()
        .filter(|relative| root.join(relative).exists())
        .map(|relative| format!("{relative} still exists"))
        .collect()
}

const CANONICAL_OWNERS: [(&str, &str); 11] = [
    ("execute_command", "src/ops/dispatch.rs"),
    ("handle_copy", "src/ops/file.rs"),
    ("handle_move", "src/ops/file.rs"),
    ("handle_mkdir", "src/ops/file.rs"),
    ("handle_delete_confirm", "src/ops/file.rs"),
    ("handle_delete", "src/ops/file.rs"),
    ("handle_create_file", "src/ops/file.rs"),
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

#[test]
fn owner_rule_reports_duplicate_and_missing_definitions() {
    let sources = [
        SourceFile::new(
            "src/ops/file.rs",
            "pub fn handle_copy(app: &mut App) -> io::Result<()> {\n    Ok(())\n}\n",
        ),
        SourceFile::new(
            "src/popup/copy.rs",
            "pub(crate) fn handle_copy<B>(app: &mut App) {}\n",
        ),
        SourceFile::new("src/ui/copy.rs", "fn handle_view() {}\n"),
    ];
    let canonical = [
        ("handle_copy", "src/ops/file.rs"),
        ("handle_view", "src/ops/viewer.rs"),
    ];
    assert_eq!(
        owner_violations(&sources, &canonical),
        [
            "`handle_copy` is defined in [\"src/ops/file.rs\", \"src/popup/copy.rs\"]; only in src/ops/file.rs",
            "`handle_view` is defined in []; only in src/ops/viewer.rs",
        ]
    );
}

#[test]
fn embedded_test_rule_reports_test_attributes_outside_test_files() {
    let sources = [
        SourceFile::new(
            "src/ops/file.rs",
            "pub fn copy() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn copies() {}\n\n    #[tokio::test]\n    async fn copies_later() {}\n}\n",
        ),
        SourceFile::new("src/ops/file/tests.rs", "#[test]\nfn copies() {}\n"),
        SourceFile::new("src/ops/hint.rs", "pub const HINT: &str = \"#[test]\";\n"),
    ];
    assert_eq!(
        embedded_test_violations(&sources),
        [
            "src/ops/file.rs:5: test attribute in production code; use a cfg(test) child file",
            "src/ops/file.rs:8: test attribute in production code; use a cfg(test) child file",
        ]
    );
}

#[test]
fn feature_selection_rule_reports_duplicates_and_feature_checks() {
    let duplicated = SourceFile::new(
        "src/bootstrap.rs",
        "#[cfg(feature = \"test-helpers\")]\npub fn run() {}\n\n#[cfg(not(feature = \"test-helpers\"))]\npub fn run() {}\n",
    );
    assert_eq!(
        feature_selection_violations(&duplicated, "run"),
        ["src/bootstrap.rs: `run` has 2 definitions; expected one"]
    );
    let branching = SourceFile::new(
        "src/editor.rs",
        "pub fn handle_edit<B>(terminal: &mut B) {\n    if cfg!(feature = \"test-helpers\") {\n        return;\n    }\n}\n",
    );
    assert_eq!(
        feature_selection_violations(&branching, "handle_edit"),
        ["src/editor.rs:1: `handle_edit` is selected by the `test-helpers` feature"]
    );
    let documented = SourceFile::new(
        "src/bootstrap.rs",
        "/// Unlike the `feature = \"test-helpers\"` seams, this is the real entry.\npub fn run() {}\n",
    );
    assert!(feature_selection_violations(&documented, "run").is_empty());
}

#[test]
fn abort_hook_rule_reads_code_order_not_comments() {
    let reversed = SourceFile::new(
        "src/bootstrap.rs",
        "fn run_session() {\n    // set_hook first, then TerminalSession::enter.\n    let session = crate::terminal::TerminalSession::enter();\n    std::panic::set_hook(Box::new(|_| {}));\n}\n",
    );
    assert_eq!(
        abort_hook_violations(&reversed),
        [
            "src/bootstrap.rs: `run_session` must call `std::panic::set_hook` (at Some((4, 4))) \
          before `TerminalSession::enter` (at Some((3, 18)))"
        ]
    );
    let imported = SourceFile::new(
        "src/bootstrap.rs",
        "use crate::terminal::TerminalSession;\nuse std::panic;\n\nfn run_session() {\n    panic::set_hook(Box::new(|_| {}));\n    let session = TerminalSession::enter();\n}\n",
    );
    assert!(abort_hook_violations(&imported).is_empty());
}

#[test]
fn obsolete_file_rule_reports_a_restored_file() {
    let directory = tempfile::tempdir().expect("temporary tree");
    fs::create_dir_all(directory.path().join("src/handlers")).expect("handlers directory");
    fs::write(directory.path().join("src/handlers/shell.rs"), "").expect("restored file");
    assert_eq!(
        present_files(directory.path(), &OBSOLETE_HANDLER_FILES),
        ["src/handlers/shell.rs still exists"]
    );
}
