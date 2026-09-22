//! Purpose: Enforce syntactic architectural dependency and ownership contracts.

use std::fs;
use std::path::{Path, PathBuf};

#[path = "module_boundaries/popup.rs"]
mod popup;

fn rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn definition_owners(symbol: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let public_definition = format!("pub fn {symbol}(");
    let crate_definition = format!("pub(crate) fn {symbol}(");
    let mut owners = files
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("read Rust source")
                .lines()
                .any(|line| {
                    let line = line.trim();
                    line.starts_with(&public_definition) || line.starts_with(&crate_definition)
                })
        })
        .map(|path| {
            path.strip_prefix(root)
                .expect("source below manifest")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    owners.sort();
    owners
}

fn function_items<'a>(source: &'a str, signature: &str) -> Vec<&'a str> {
    source
        .match_indices(signature)
        .map(|(signature_start, _)| {
            let item_start = source[..signature_start]
                .rfind("\n\n")
                .map_or(0, |boundary| boundary + 2);
            let body_start = source[signature_start..].find('{').map_or_else(
                || panic!("missing body for `{signature}`"),
                |offset| signature_start + offset,
            );

            let mut depth = 0;
            for (offset, character) in source[body_start..].char_indices() {
                match character {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                    }
                    _ => {}
                }
                if depth == 0 {
                    let item_end = body_start + offset + character.len_utf8();
                    return &source[item_start..item_end];
                }
            }

            panic!("unclosed body for `{signature}`");
        })
        .collect()
}

#[test]
fn operations_have_one_canonical_owner() {
    for (symbol, owner) in [
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
    ] {
        assert_eq!(
            definition_owners(symbol),
            vec![owner.to_owned()],
            "{symbol}"
        );
    }
}

#[test]
fn obsolete_handler_implementation_files_are_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/handlers/dispatch.rs",
        "src/handlers/dispatch/tests.rs",
        "src/handlers/file_ops.rs",
        "src/handlers/mcp.rs",
        "src/handlers/shell.rs",
    ] {
        assert!(!root.join(relative).exists(), "{relative} still exists");
    }
}

#[test]
fn abort_hook_precedes_terminal_activation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/bootstrap.rs")).expect("read bootstrap.rs");
    let main_impl = function_items(&lib, "fn run_session")
        .into_iter()
        .next()
        .expect("run_session");
    let hook = main_impl.find("set_hook").expect("abort hook");
    let activation = main_impl
        .find("TerminalSession::enter")
        .expect("terminal activation");

    assert!(
        hook < activation,
        "the abort hook must be active before terminal setup can mutate process state"
    );
}

#[test]
fn handlers_keeps_stable_operation_signatures() {
    use funny_file_manager::{handlers, models::App};
    use std::io;

    let _: fn(&mut App) -> io::Result<()> = handlers::handle_copy;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_move;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_mkdir;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_delete_confirm;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_delete;
    let _: fn(&mut App) -> io::Result<()> = handlers::handle_view;
    let _: fn(&mut App) -> io::Result<()> = handlers::open_mcp_status;
    let _: fn(&mut App, &str) -> io::Result<()> = handlers::execute_command;
}

#[test]
fn handlers_keeps_shell_module_path() {
    #[allow(unused_imports)]
    use funny_file_manager::handlers::shell;
}

#[test]
fn test_helpers_do_not_select_run_implementation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/bootstrap.rs")).expect("read bootstrap.rs");
    let run_items = function_items(&lib, "pub fn run()");
    assert_eq!(run_items.len(), 1);
    assert!(run_items
        .iter()
        .all(|item| !item.contains("feature = \"test-helpers\"")));

    let main_items = function_items(&lib, "fn main_impl");
    assert_eq!(main_items.len(), 1);
    assert!(main_items
        .iter()
        .all(|item| !item.contains("feature = \"test-helpers\"")));
}

#[test]
fn test_helpers_do_not_select_editor_implementation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let editor = fs::read_to_string(root.join("src/editor.rs")).expect("read editor.rs");
    let editor_items = function_items(&editor, "pub fn handle_edit<");
    assert_eq!(editor_items.len(), 1);
    assert!(editor_items
        .iter()
        .all(|item| !item.contains("feature = \"test-helpers\"")));
}

fn is_test_source(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
        || path.file_stem().is_some_and(|stem| {
            let stem = stem.to_string_lossy();
            stem.starts_with("tests") || stem.starts_with("test_") || stem.ends_with("_tests")
        })
}

fn compact_code(path: &Path) -> String {
    fs::read_to_string(path)
        .expect("read Rust source")
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect()
}

#[test]
fn unit_test_bodies_live_in_separate_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for directory in ["src", "mcp/src", "colorer-sys/src"] {
        rust_files(&root.join(directory), &mut files);
    }
    for path in files.iter().filter(|path| !is_test_source(path)) {
        assert!(
            !compact_code(path).contains("#[test]"),
            "{} embeds test bodies in production code; use a cfg(test) child file",
            path.display()
        );
    }
}

#[test]
fn crate_root_and_compatibility_facade_only_wire_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["src/lib.rs", "src/handlers/mod.rs"] {
        let source = fs::read_to_string(root.join(relative)).expect("read facade");
        for line in source.lines().map(str::trim) {
            assert!(
                line.starts_with("//") || !line.contains("fn "),
                "{relative} owns behavior: {line}"
            );
        }
    }
    let handlers = compact_code(&root.join("src/handlers/mod.rs"));
    assert!(
        !handlers.contains("#[cfg(test)]"),
        "compatibility facade must not own operation tests or private test reexports"
    );
}

#[test]
fn presentation_has_no_storage_or_operation_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("src/ui.rs"), root.join("src/action.rs")];
    rust_files(&root.join("src/ui"), &mut files);
    rust_files(&root.join("src/action"), &mut files);
    for path in files.iter().filter(|path| !is_test_source(path)) {
        let code = compact_code(path);
        for forbidden in [
            "std::fs",
            "std::{fs",
            "std::process",
            "std::{process",
            "crate::directory",
            "crate::diskfree",
            "crate::ops",
            "crate::handlers",
            "crate::popup",
            "crate::editor",
            "crate::terminal",
        ] {
            assert!(
                !code.contains(forbidden),
                "{} depends on {forbidden}; rendering and key resolution must read state",
                path.display()
            );
        }
    }
}
