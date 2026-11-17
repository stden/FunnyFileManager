//! Purpose: Enforce source-size limits and reject duplicate handler definitions.
//!
//! Hard limit: no Rust or Python source in the repository may exceed
//! [`MAX_LINES`] lines — application, tests, benches, examples, the MCP server,
//! colorer-sys and the developer scripts alike. Split by responsibility (see
//! `docs/knowledge-base/02-architecture/module-structure.md`). [`EXEMPT`] names
//! the only files allowed past it, each with its reason, and may only shrink.

use std::fs;
use std::path::Path;

#[path = "file_line_limit_test/sources.rs"]
mod sources;

use sources::{collect_rs_files, read_sources, repository_sources};

/// Hard cap enforced by this test.
const MAX_LINES: usize = 200;

/// The files allowed past [`MAX_LINES`], and why.
const EXEMPT: [(&str, &str); 1] = [(
    "src/status.rs",
    "one `pub enum Status` of 76 documented variants; an enum cannot span files, and \
     grouping it into nested enums would change 448 call sites",
)];

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
fn no_source_exceeds_line_limit() {
    let sources = repository_sources();
    assert!(sources.len() > 1000, "found only {} sources", sources.len());

    let mut offenders: Vec<String> = sources
        .iter()
        .filter(|(path, lines)| *lines > MAX_LINES && !EXEMPT.iter().any(|(name, _)| name == path))
        .map(|(path, lines)| format!("{path} ({lines} lines)"))
        .collect();
    offenders.sort();

    assert_eq!(
        offenders.len(),
        0,
        "files exceed the {MAX_LINES}-line limit — split them into modules:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_exemption_is_still_needed() {
    let sources = repository_sources();
    let stale: Vec<&str> = EXEMPT
        .iter()
        .filter(|(name, _)| {
            !sources
                .iter()
                .any(|(path, lines)| path == name && *lines > MAX_LINES)
        })
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(stale.len(), 0,
        "exempt files that no longer exceed the limit (or no longer exist) — drop them from EXEMPT: {stale:?}"
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

    assert_eq!(
        offenders.len(),
        0,
        "each handler must be defined exactly once under src/ — a second copy is dead code \
         that no lint can see:\n{}",
        offenders.join("\n")
    );
}
