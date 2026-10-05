//! Purpose: Find programs on the host's PATH: a client's own command, and
//! the `ffm-mcp` server beside `ffm` or on PATH.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::mcp_clients::{Client, Host, ServerBinary, ServerState, SERVER_NAME};

/// The first `name` in a PATH folder that can be run, as an absolute path.
/// Relative PATH entries are skipped: they name wherever FFM happens to be.
#[must_use]
pub fn find_program(host: &Host, name: &str) -> Option<PathBuf> {
    host.search_path
        .iter()
        .filter(|dir| dir.is_absolute())
        .find_map(|dir| executable_in(host, dir, name))
}

/// The first program the client table lists for `client` that is on PATH.
#[must_use]
pub fn client_program(host: &Host, client: &Client) -> Option<PathBuf> {
    client
        .programs
        .iter()
        .find_map(|program| find_program(host, program))
}

fn executable_in(host: &Host, dir: &Path, name: &str) -> Option<PathBuf> {
    host.program_suffixes
        .iter()
        .map(|suffix| dir.join(format!("{name}{suffix}")))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    /// Any of the owner, group and other execute bits.
    const EXECUTE: u32 = 0o111;
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & EXECUTE != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// `ffm-mcp` next to the running `ffm` — where `cargo install --path mcp`
/// leaves it — else on PATH. The path is used as found: canonicalizing it
/// would put a `\\?\` prefix into a Windows config.
#[must_use]
pub fn locate_server(host: &Host) -> ServerState {
    let beside_ffm = host
        .exe_dir
        .as_deref()
        .and_then(|dir| executable_in(host, dir, SERVER_NAME));
    match beside_ffm.or_else(|| find_program(host, SERVER_NAME)) {
        None => ServerState::Missing,
        Some(path) => {
            ServerBinary::new(path).map_or_else(ServerState::NotUnicode, ServerState::Found)
        }
    }
}
