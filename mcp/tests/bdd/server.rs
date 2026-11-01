//! Purpose: Runs one tool call against a freshly spawned `ffm-mcp` over
//! JSON-RPC stdio, then closes its input and waits for a clean exit so the
//! LLVM profiling runtime writes the coverage the MCP CI gate measures.

use crate::world::McpWorld;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long a tool call may take before the harness gives up on a response.
const RESPONSE_DEADLINE: Duration = Duration::from_secs(5);

/// How long the server gets to shut itself down after its stdin is closed.
/// Short on purpose: the server has already answered by the time we wait, so
/// exiting is only a matter of unwinding `serve()`.
const EXIT_DEADLINE: Duration = Duration::from_secs(5);

/// Gap between `try_wait` polls while the server shuts down.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

fn forward_responses(reader: impl BufRead, sender: std::sync::mpsc::Sender<String>) {
    for line in reader
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
    {
        // The caller stops listening once it has its answer. The rest is
        // still drained, unread, so the server never writes into a closed pipe.
        drop(sender.send(line));
    }
}

impl McpWorld {
    #[expect(
        clippy::expect_used,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    pub fn call_tool(&mut self, name: &str, args: Value) {
        self.last_result = None;
        self.last_error = None;
        self.server_exit = None;

        let mut child = self.spawn_server();

        // Owned, not borrowed: the pipe has to be dropped explicitly once the
        // exchange is over so the server sees EOF (see the `drop` below).
        let mut stdin = child.stdin.take().expect("stdin");
        send_tool_request(&mut stdin, name, args);

        // Read responses with timeout
        let stdout = child.stdout.take().expect("stdout");
        let reader = BufReader::new(stdout);
        let mut found = false;

        // Use a thread with timeout to avoid hanging
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || forward_responses(reader, tx));

        let deadline = Instant::now() + RESPONSE_DEADLINE;
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(line) if self.accept_tool_response(&line) => {
                    found = true;
                    break;
                }
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }

        // Closing the request pipe is what ends the session: the stdio
        // transport reads EOF, `serve()` returns, and the server leaves
        // through `main` instead of being shot. That is not just tidiness —
        // the LLVM profiling runtime writes its .profraw from an atexit hook,
        // and SIGKILL never runs it, so the previous `child.kill()` left
        // src/main.rs at 0% coverage even though every scenario drives it.
        drop(stdin);
        self.server_exit = Some(wait_for_exit(&mut child));
        // The reader thread ends at stdout EOF, which the exit above guarantees.
        // It cannot panic, so joining it has no error to report.
        drop(handle.join());

        if !found && self.last_error.is_none() {
            self.last_error = Some("no response from server".to_owned());
        }
    }

    #[expect(
        clippy::panic,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    fn spawn_server(&self) -> Child {
        let binary = binary_path();
        // The server confines every path argument to this root; each scenario
        // runs inside its own temp directory.
        let root = self
            .temp_dir
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |d| d.path().to_path_buf());
        // HOME points at the same temp dir so the tools that read the home
        // directory — bookmarks in ~/.ffm, AWS profiles in ~/.aws — see only
        // what the scenario put there and never touch the real user's files.
        let child = Command::new(&binary)
            .env("FFM_MCP_ROOT", &root)
            .env("FFM_MCP_ALLOW_KDBX", "1")
            .env("HOME", &root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {}", binary.display(), e));

        child
    }
}

/// Reap the server, giving it a bounded chance to exit on its own first.
///
/// Polling rather than a blocking `wait()` keeps one wedged server from
/// hanging the whole suite; the kill is the fallback, not the norm. Either
/// branch reaps the child, so no zombies accumulate across the suite.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + EXIT_DEADLINE;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) => std::thread::sleep(EXIT_POLL_INTERVAL),
            Err(e) => panic!("try_wait on server: {e}"),
        }
    }
    // A server that exits between the last poll and the kill cannot be
    // killed; the `wait` below reaps it either way.
    drop(child.kill());
    child.wait().expect("reap killed server")
}

fn binary_path() -> PathBuf {
    // Cargo supplies the exact platform-specific executable path to
    // integration tests. Deriving it from `current_exe()` broke both on
    // Windows (`.exe`) and with Cargo's isolated artifact directories.
    PathBuf::from(env!("CARGO_BIN_EXE_ffm-mcp"))
}

#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
fn send_tool_request(stdin: &mut impl Write, name: &str, args: Value) {
    let msgs = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2024-11-05","capabilities":{},
            "clientInfo":{"name":"bdd","version":"0.1"}
        }}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
            "name": name, "arguments": args
        }}),
    ];

    for msg in &msgs {
        writeln!(stdin, "{msg}").expect("write to stdin");
    }
    stdin.flush().expect("flush stdin");
}
