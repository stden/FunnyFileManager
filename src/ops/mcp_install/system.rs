//! Purpose: The real machine for the installer — home folder, the client
//! table's override variables, PATH, PATHEXT and the running executable —
//! and the runner that starts a real client CLI.
//!
//! Only the production entry point calls [`system_host`], and only for live
//! input: tests and replayed recordings keep the inert default host.

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use crate::mcp_clients::{env_vars, Captured, CliCall, Exit, Host, Runner};

/// What `cmd.exe` runs when PATHEXT is unset.
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// The machine FFM is running on.
#[must_use]
pub fn system_host() -> Host {
    Host {
        home: home_dir(),
        env: env_vars()
            .into_iter()
            .filter_map(|var| non_empty(var).map(|value| (var, value)))
            .collect(),
        search_path: env::var_os("PATH")
            .map(|path| env::split_paths(&path).collect())
            .unwrap_or_default(),
        program_suffixes: program_suffixes(),
        exe_dir: env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf)),
        runner: Arc::new(SystemRunner),
    }
}

fn non_empty(var: &str) -> Option<std::ffi::OsString> {
    env::var_os(var).filter(|value| !value.is_empty())
}

/// Where Node's `os.homedir()` (Claude) and Rust's `dirs::home_dir()`
/// (Codex) look, in their order.
#[cfg(windows)]
const HOME_VARS: [&str; 2] = ["USERPROFILE", "HOME"];
#[cfg(not(windows))]
const HOME_VARS: [&str; 1] = ["HOME"];

fn home_dir() -> Option<PathBuf> {
    HOME_VARS.into_iter().find_map(non_empty).map(PathBuf::from)
}

#[cfg(windows)]
fn program_suffixes() -> Vec<String> {
    parse_pathext(env::var("PATHEXT").ok().as_deref())
}

/// A Unix program is found by its exact name.
#[cfg(not(windows))]
fn program_suffixes() -> Vec<String> {
    vec![String::new()]
}

/// PATHEXT's extensions, lower-cased: what a bare program name may end in on
/// Windows, so the npm shims `claude.cmd` and `codex.cmd` are found.
#[cfg_attr(
    all(not(windows), not(test)),
    expect(
        dead_code,
        reason = "only Windows reads PATHEXT; the parser is pure and tested on every platform"
    )
)]
#[must_use]
pub fn parse_pathext(pathext: Option<&str>) -> Vec<String> {
    let listed = pathext
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_PATHEXT);
    listed
        .split(';')
        .map(str::trim)
        .filter(|suffix| !suffix.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Runs a client CLI with no terminal: stdin empty, output captured, killed
/// at its timeout.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, call: &CliCall) -> std::io::Result<Captured> {
        let mut command = Command::new(&call.program);
        command.args(&call.args).current_dir(&call.cwd);
        let output = crate::ops::menu::process::capture_with_timeout(
            &mut command,
            call.timeout,
            &env::temp_dir(),
        )?;
        Ok(Captured {
            exit: if output.status.success() {
                Exit::Success
            } else {
                Exit::Failure(output.status.code())
            },
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests;
