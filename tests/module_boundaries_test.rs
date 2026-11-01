//! Purpose: Enforce architectural dependency and ownership contracts on parsed sources.
//!
//! Each rule is a function from sources to violation messages. The real tree
//! must produce none; the fixtures under `module_boundaries/` plant one
//! violation per rule and pin its exact message. How paths are resolved, and
//! what the checker cannot see, is documented in `architecture/mod.rs`.

mod architecture;

#[path = "module_boundaries/layers.rs"]
mod layers;
#[path = "module_boundaries/mcp.rs"]
mod mcp;
#[path = "module_boundaries/ownership.rs"]
mod ownership;
#[path = "module_boundaries/popup.rs"]
mod popup;
#[path = "module_boundaries/resolution.rs"]
mod resolution;
#[path = "module_boundaries/space.rs"]
mod space;

use architecture::{package_sources, SourceFile, Violation};
use std::path::Path;

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn tui_sources() -> Vec<SourceFile> {
    package_sources(repository(), "")
}

fn messages(violations: &[Violation]) -> Vec<String> {
    violations.iter().map(ToString::to_string).collect()
}

/// Compile-time contract: the facade keeps the historical operation signatures.
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

/// Compile-time contract: the historical module path still resolves.
#[test]
fn handlers_keeps_shell_module_path() {
    #[expect(
        unused_imports,
        reason = "the contract is that the path resolves, not that it is used"
    )]
    use funny_file_manager::handlers::shell;
}
