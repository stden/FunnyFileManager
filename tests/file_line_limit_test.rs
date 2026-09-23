//! Purpose: Enforce source-size limits and reject duplicate handler definitions.
//!
//! Hard limit: no file under `src/` may exceed [`MAX_LINES`] lines.
//! Convention: target at most 300 lines, split by responsibility
//! (see `docs/knowledge-base/02-architecture/module-structure.md`).

use std::fs;
use std::path::{Path, PathBuf};

/// Hard cap enforced by this test.
const MAX_LINES: usize = 1000;

/// Handlers that once existed twice: `src/ops.rs` held a byte-identical fork of
/// `handlers::mcp::open_mcp_status` and a stale copy of
/// `handlers::file_ops::handle_view`. Nothing called the fork, `dead_code`
/// cannot fire on a `pub` item in a lib crate, and clippy stayed silent — so
/// maintainers kept editing the copy nobody ran. Pin the count at one.
const SINGLE_DEFINITION_HANDLERS: &[&str] = &[
    "open_mcp_status",
    "handle_view",
    "handle_copy",
    "handle_move",
    "handle_mkdir",
    "handle_delete",
    "execute_command",
    "process_popup_input",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn read_sources(files: Vec<PathBuf>) -> Vec<(PathBuf, String)> {
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read Rust source {}: {error}", path.display()));
            (path, text)
        })
        .collect()
}

#[test]
#[should_panic(expected = "read Rust source")]
fn missing_rust_source_fails_the_guard() {
    let temp = tempfile::TempDir::new().unwrap();
    read_sources(vec![temp.path().join("missing.rs")]);
}

#[test]
#[should_panic(expected = "read Rust source")]
fn invalid_utf8_source_fails_the_guard() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("invalid.rs");
    fs::write(&path, [0xff]).unwrap();
    read_sources(vec![path]);
}

#[test]
fn no_src_file_exceeds_line_limit() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    assert!(!files.is_empty(), "no .rs files found under src/");

    let mut offenders: Vec<String> = read_sources(files)
        .iter()
        .filter_map(|(path, text)| {
            let lines = text.lines().count();
            (lines > MAX_LINES).then(|| format!("{} ({lines} lines)", path.display()))
        })
        .collect();
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "files exceed the {MAX_LINES}-line limit — split them into modules:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn no_handler_is_defined_twice() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    files.sort();

    let sources = read_sources(files);

    let mut offenders = Vec::new();
    for name in SINGLE_DEFINITION_HANDLERS {
        let signature = format!("pub fn {name}(");
        let defined_in: Vec<String> = sources
            .iter()
            .filter(|(_, text)| text.contains(&signature))
            .map(|(path, _)| path.display().to_string())
            .collect();
        if defined_in.len() != 1 {
            offenders.push(format!("{name}: {}", defined_in.join(", ")));
        }
    }

    assert!(
        offenders.is_empty(),
        "each handler must be defined exactly once under src/ — a second copy is dead code \
         that no lint can see:\n{}",
        offenders.join("\n")
    );
}
