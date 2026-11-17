//! Purpose: The real machine for the installer — home folder, the client
//! table's override variables, PATH, PATHEXT and the running executable —
//! and the runner that starts a real client CLI.
//!
//! Only the production entry point calls [`system_host`], and only for live
//! input: tests and replayed recordings keep the inert default host.

use std::{env, io, path::Path, process::Command, sync::Arc};

use crate::mcp_clients::{env_vars, Captured, CliCall, Exit, Host, Runner};

/// What `cmd.exe` runs when PATHEXT is unset.
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// The machine FFM is running on.
#[must_use]
pub fn system_host() -> Host {
    Host {
        // Where Node's `os.homedir()` (Claude) and Rust's `dirs::home_dir()`
        // (Codex) look, in their order.
        home: crate::config::user_home(),
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
    fn run(&self, call: &CliCall) -> io::Result<Captured> {
        let mut command = build_command(call)?;
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

fn build_command(call: &CliCall) -> io::Result<Command> {
    let mut command = Command::new(&call.program);
    command.current_dir(&call.cwd);
    if is_batch_file(&call.program) && call.args.iter().any(|arg| arg.contains('"')) {
        for arg in &call.args {
            if arg.chars().any(|c| {
                c.is_control() || matches!(c, '&' | '|' | '<' | '>' | '(' | ')' | '^' | '%' | '!')
            }) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "batch argument contains cmd.exe metacharacters",
                ));
            }
            let quoted = quote_c_arg(arg);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.raw_arg(&quoted);
            }
            #[cfg(not(windows))]
            command.arg(&quoted);
        }
        return Ok(command);
    }
    command.args(&call.args);
    Ok(command)
}

fn is_batch_file(program: &Path) -> bool {
    program
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
}

fn quote_c_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_owned();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        if ch == '\\' {
            backslashes += 1;
        } else if ch == '"' {
            for _ in 0..=backslashes * 2 {
                out.push('\\');
            }
            out.push('"');
            backslashes = 0;
        } else {
            for _ in 0..backslashes {
                out.push('\\');
            }
            out.push(ch);
            backslashes = 0;
        }
    }
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests;
