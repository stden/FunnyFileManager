//! Purpose: Own one real MCP subprocess with private, deadline-bounded JSON-RPC pipes.

use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::BufReader,
    process::{Child, ChildStdin, ChildStdout, Command},
};

#[path = "client/rpc.rs"]
mod rpc;
#[path = "client/session.rs"]
mod session;

#[derive(Clone, Copy)]
pub(super) enum Access {
    Application,
    Disabled,
    Readonly,
}

#[derive(Clone)]
pub(super) struct SessionId(String);

pub(super) struct Client {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    sequence: u64,
}

impl Client {
    pub(super) async fn spawn(root: &Path, access: Access) -> Self {
        let mut child = command(root, access).spawn().expect("spawn built ffm-mcp");
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().expect("private stdout"));
        let mut client = Self {
            child,
            input,
            output,
            sequence: 0,
        };
        let hello = client
            .rpc(
                "initialize",
                json!({"protocolVersion":"2024-11-05",
            "capabilities":{},"clientInfo":{"name":"application-e2e","version":"1"}}),
            )
            .await;
        assert!(hello.get("result").is_some(), "{hello}");
        client
            .send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await;
        client
    }

    pub(super) async fn close(mut self) {
        self.input.take();
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .expect("MCP exits after stdin EOF")
            .expect("reap MCP server");
        assert!(status.success(), "MCP server exited with {status}");
    }
}

fn command(root: &Path, access: Access) -> Command {
    let binary = artifact();
    let allowed = matches!(access, Access::Application | Access::Readonly);
    let binary = if matches!(access, Access::Application) {
        binary
    } else {
        root.join("absent-ffm")
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_ffm-mcp"));
    command
        .env("FFM_MCP_ROOT", root)
        .env("FFM_MCP_FFM_BINARY", binary)
        .env("FFM_MCP_ALLOW_APPLICATION", flag(allowed))
        .env("FFM_MCP_READONLY", flag(matches!(access, Access::Readonly)))
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("XDG_CONFIG_HOME", root)
        .env("TMPDIR", root)
        .env("TMP", root)
        .env("TEMP", root)
        .env("FFM_LOG", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}

fn flag(enabled: bool) -> &'static str {
    if enabled {
        "1"
    } else {
        "0"
    }
}

fn artifact() -> PathBuf {
    let artifact = std::env::var_os("FFM_APPLICATION_TEST_BINARY")
        .expect("explicit E2E run requires FFM_APPLICATION_TEST_BINARY pointing to built ffm");
    assert!(
        Path::new(&artifact).is_absolute(),
        "FFM test artifact path must be absolute"
    );
    let binary = Path::new(&artifact)
        .canonicalize()
        .expect("built FFM artifact exists");
    assert!(binary.is_file(), "FFM artifact must be an executable file");
    binary
}
