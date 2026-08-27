//! Intentionally syntactic architectural dependency and ownership contracts.

use std::fs;
use std::path::{Path, PathBuf};

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
                        if depth == 0 {
                            let item_end = body_start + offset + character.len_utf8();
                            return &source[item_start..item_end];
                        }
                    }
                    _ => {}
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
fn popup_does_not_depend_on_compatibility_handlers() {
    let popup_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/popup.rs");
    let popup = fs::read_to_string(&popup_path).expect("read popup.rs");
    let mut compact_source = String::new();
    for line in popup.lines() {
        let source = line.split_once("//").map_or(line, |(source, _)| source);
        compact_source.extend(
            source
                .chars()
                .filter(|character| !character.is_whitespace()),
        );
    }

    let forbidden = [
        "crate::handlers",
        "crate::{handlers",
        "super::handlers",
        "super::{handlers",
    ];
    let violations = forbidden
        .into_iter()
        .filter(|dependency| compact_source.contains(dependency))
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "{} contains forbidden dependencies: {violations:?}",
        popup_path.display()
    );
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
    let main_impl = function_items(&lib, "fn main_impl")
        .into_iter()
        .next()
        .expect("main_impl");
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
