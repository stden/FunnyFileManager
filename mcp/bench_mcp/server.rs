//! Purpose: Finds and starts the `ffm-mcp` server the benchmark measures,
//! and exchanges its line-delimited JSON-RPC messages.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Start the server confined to `root`, with its stdin and stdout piped to us.
#[expect(
    clippy::expect_used,
    reason = "a benchmark without its server has nothing to measure: abort"
)]
#[expect(
    clippy::unwrap_used,
    reason = "both streams are `Stdio::piped()`, so the child always has them"
)]
pub fn start_server(binary: &Path, root: &Path) -> (Child, ChildStdin, ChildStdout) {
    let mut child = Command::new(binary)
        // The server confines every path to this root.
        .env("FFM_MCP_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start ffm-mcp");
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    (child, stdin, stdout)
}

#[expect(
    clippy::unwrap_used,
    reason = "a benchmark that has lost its server has nothing left to measure: abort"
)]
pub fn send(stdin: &mut impl Write, msg: &str) {
    // rmcp uses line-delimited JSON (no Content-Length header)
    stdin.write_all(msg.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "a benchmark that has lost its server has nothing left to measure: abort"
)]
pub fn recv(reader: &mut impl BufRead) -> String {
    // Read one JSON line
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line
}

/// Find server artifacts beside this Cargo example, including custom targets.
/// The cwd-relative fallback also keeps the standalone rustc workflow working.
pub fn server_candidates(executable: &Path) -> Vec<PathBuf> {
    let name = format!("ffm-mcp{}", std::env::consts::EXE_SUFFIX);
    let mut candidates = Vec::new();
    if let Some(examples) = executable
        .parent()
        .filter(|path| path.ends_with("examples"))
    {
        if let Some(target) = examples.parent().and_then(Path::parent) {
            candidates.push(target.join("release").join(&name));
            candidates.push(target.join("debug").join(&name));
        }
    }
    candidates.push(Path::new("target/release").join(&name));
    candidates.push(Path::new("target/debug").join(name));
    candidates
}

pub fn locate_server() -> Result<PathBuf, &'static str> {
    std::env::var_os("FFM_MCP_BIN")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .into_iter()
                .flat_map(|executable| server_candidates(&executable))
                .find(|candidate| candidate.is_file())
        })
        .ok_or("ffm-mcp binary not found. Run 'cargo build --release' first or set FFM_MCP_BIN.")
}
