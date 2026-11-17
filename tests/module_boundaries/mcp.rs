//! Purpose: Keep the MCP server a separate process whose file access stays root-confined.
//!
//! What is enforced is what the source shows:
//! - neither crate depends on the other (their lockfiles resolve every
//!   dependency, of any kind and for any target);
//! - the only MCP source the TUI compiles is `mcp/src/tool_names.rs`, and that
//!   file names nothing else in its crate;
//! - only the server composition (`main.rs`) calls the confinement API in
//!   `paths.rs`, and every extracted tool handed the server validates its path
//!   with `server.safe_path*` before anything else;
//! - modules that name `std::fs` are listed in `mcp/filesystem.rs` with how
//!   their paths stay inside the root.
//!
//! Not seen: `Path` methods that reach the filesystem without naming `std::fs`
//! (`exists`, `is_dir`, `canonicalize`, `read_dir`, `metadata`).

use super::architecture::{
    dependencies, forbidden_dependencies, included_files, package_sources, starts_with_path,
    SourceFile,
};
use super::messages;
use std::fs;
use syn::visit::{self, Visit};
use syn::{Block, ExprMethodCall, FnArg, Ident, Item, Signature};

#[path = "mcp/cross_crate.rs"]
mod cross_crate;
#[path = "mcp/filesystem.rs"]
mod filesystem;
#[path = "mcp/shared_source.rs"]
mod shared_source;
#[path = "mcp/tool_validation.rs"]
mod tool_validation;
use cross_crate::cross_dependencies;
use filesystem::{confinement_api_violations, filesystem_violations, Idents};
use shared_source::shared_source_violations;
use tool_validation::unvalidated_tool_violations;

/// The server composition: it resolves the root and owns `safe_path*`.
const SERVER: &str = "mcp/src/main.rs";

const SHARED_SOURCE: &str = "mcp/src/tool_names.rs";

#[expect(
    clippy::panic,
    reason = "test support: a file the checker cannot read fails the test that asked for it"
)]
fn read(relative: &str) -> String {
    fs::read_to_string(super::repository().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn mcp_sources() -> Vec<SourceFile> {
    package_sources(super::repository(), "mcp")
}

#[test]
fn tui_and_mcp_crates_do_not_depend_on_each_other() {
    let violations = cross_dependencies(&read("Cargo.lock"), &read("mcp/Cargo.lock"));
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn tui_compiles_only_the_import_free_tool_list() {
    let mcp = mcp_sources();
    let tool_names = mcp
        .iter()
        .find(|file| file.relative == SHARED_SOURCE)
        .expect("tool list");
    let violations = shared_source_violations(&super::tui_sources(), tool_names);
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn mcp_file_access_goes_through_the_root_confinement() {
    let sources = mcp_sources();
    assert_eq!(filesystem_violations(&sources), Vec::<String>::new());
    assert_eq!(confinement_api_violations(&sources), Vec::<String>::new());
    assert_eq!(unvalidated_tool_violations(&sources), Vec::<String>::new());
}
