//! Purpose: The host: system information and where an executable on PATH is.

use super::json_response;
use crate::FfmServer;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;
#[cfg(unix)]
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WhichParams {
    /// Executable name to look for, without a directory component
    pub name: String,
}

#[tool_router(router = system_tools, vis = "pub(super)")]
impl FfmServer {
    /// Get system information: hostname, OS, architecture, home directory.
    #[tool(
        description = "Get system information: hostname, OS, architecture, home directory, current directory."
    )]
    pub(crate) fn system_info() -> CallToolResult {
        let mut info = serde_json::Map::new();
        info.insert("os".to_owned(), json!(std::env::consts::OS));
        info.insert("arch".to_owned(), json!(std::env::consts::ARCH));
        info.insert("family".to_owned(), json!(std::env::consts::FAMILY));

        if let Ok(cwd) = std::env::current_dir() {
            let cwd = cwd.display().to_string();
            info.insert("current_directory".to_owned(), json!(cwd));
        }
        if let Some(home) = dirs_home() {
            let home = home.display().to_string();
            info.insert("home_directory".to_owned(), json!(home));
        }

        // Hostname
        #[cfg(unix)]
        if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
            info.insert("hostname".to_owned(), json!(hostname.trim()));
        }

        // Disk space for root/home
        #[cfg(unix)]
        if let Some(line) = root_disk_usage() {
            info.insert("root_disk".to_owned(), json!(line));
        }

        json_response(&serde_json::Value::Object(info))
    }

    /// Where a command would come from, before running it.
    #[tool(
        description = "Find an executable on PATH, as `which`/`where` does. Returns every match in PATH order, so a shadowed binary is visible. On Unix only a file with an execute bit counts."
    )]
    pub(crate) fn which(
        Parameters(params): Parameters<WhichParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.name.is_empty() || params.name.contains(['/', '\\']) {
            return Err(McpError::invalid_params(
                "name must be a bare executable name, with no directory part".to_owned(),
                None,
            ));
        }
        let path_list = std::env::var_os("PATH").unwrap_or_default();
        let found = which_all(&params.name, &path_list);

        let result = json!({
            "name": params.name,
            "found": !found.is_empty(),
            "path": found.first().map(|p| p.display().to_string()),
            "all": found
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
        });
        Ok(json_response(&result))
    }
}

/// Every `name` in `path_list` (a PATH-style value), in order, so a shadowed
/// executable is visible.
///
/// On Windows a bare name is tried against each PATHEXT suffix as well, which
/// is how the shell itself resolves `cargo` to `cargo.exe`.
pub fn which_all(name: &str, path_list: &std::ffi::OsStr) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in std::env::split_paths(path_list) {
        for candidate in candidate_names(name) {
            let file = directory.join(&candidate);
            if file.is_file() && is_executable(&file) && !found.contains(&file) {
                found.push(file);
            }
        }
    }
    found
}

/// Whether `path` carries any execute bit. That is a permission-bit test,
/// not an `access(X_OK)` answer for the current user — but without it a plain
/// file named like the command — a README, a data file — shadows the real
/// binary, and `which` names something no shell would ever run.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

/// Windows resolves a command by its PATHEXT suffix, not by a permission bit.
#[cfg(windows)]
fn is_executable(_path: &Path) -> bool {
    true
}

/// The file names a bare command could have on this platform.
fn candidate_names(name: &str) -> Vec<String> {
    use std::env;
    if !cfg!(windows) {
        return vec![name.to_owned()];
    }
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    let mut names = vec![name.to_owned()];
    names.extend(
        extensions
            .split(';')
            .map(str::trim)
            .filter(|extension| !extension.is_empty())
            .map(|extension| format!("{name}{extension}")),
    );
    names
}

pub fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// The root filesystem's line of `df -h /`, trimmed; `None` when `df` fails.
#[cfg(unix)]
fn root_disk_usage() -> Option<String> {
    let output = std::process::Command::new("df")
        .args(["-h", "/"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().nth(1).map(|line| line.trim().to_owned())
}
