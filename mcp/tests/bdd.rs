//! Purpose: Runs BDD coverage for the MCP server JSON-RPC tool behavior.
//!
#![allow(
    clippy::unused_async,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber dictates async steps over `&mut World`; the harness passes JSON by value"
)]

use cucumber::{given, then, when, World as _};
use keepass::config::KdfConfig;
use keepass::db::fields;
use keepass::{Database, DatabaseKey};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// How long a tool call may take before the harness gives up on a response.
const RESPONSE_DEADLINE: Duration = Duration::from_secs(5);

/// How long the server gets to shut itself down after its stdin is closed.
/// Short on purpose: the server has already answered by the time we wait, so
/// exiting is only a matter of unwinding `serve()`.
const EXIT_DEADLINE: Duration = Duration::from_secs(5);

/// Gap between `try_wait` polls while the server shuts down.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

// ─── World ──────────────────────────────────────────────────────

#[derive(Debug, Default, cucumber::World)]
struct McpWorld {
    temp_dir: Option<TempDir>,
    /// A second tree the server root does *not* cover. Nothing here may ever
    /// be read, written or listed by a tool — it is the "outside" that a
    /// symlink planted inside the root would have to reach.
    outside_dir: Option<TempDir>,
    last_result: Option<Value>,
    last_error: Option<String>,
    /// How the server process of the last tool call terminated.
    server_exit: Option<ExitStatus>,
}

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
    fn temp_path(&self) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("no temp dir")
            .path()
            .to_path_buf()
    }

    fn resolve(&self, relative: &str) -> String {
        self.temp_path().join(relative).display().to_string()
    }

    /// The out-of-root tree, created on first use.
    #[expect(
        clippy::expect_used,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    fn outside_path(&mut self) -> PathBuf {
        self.outside_dir
            .get_or_insert_with(|| TempDir::new().expect("create outside dir"))
            .path()
            .to_path_buf()
    }

    #[expect(
        clippy::expect_used,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )]
    fn call_tool(&mut self, name: &str, args: Value) {
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
            .env("HOME", &root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {}", binary.display(), e));

        child
    }

    /// Ignore initialization and malformed lines, accepting only this tool call's id.
    fn accept_tool_response(&mut self, line: &str) -> bool {
        let Ok(response) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        if response.get("id") != Some(&json!(2)) {
            return false;
        }
        if let Some(error) = response.get("error") {
            self.last_error = Some(error["message"].as_str().unwrap_or("unknown").to_owned());
        } else if let Some(result) = response.get("result") {
            self.record_tool_result(result);
        }
        true
    }

    fn record_tool_result(&mut self, result: &Value) {
        let text = result
            .get("content")
            .and_then(|content| content.get(0))
            .and_then(|content| content.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if result.get("isError") == Some(&json!(true)) {
            self.last_error = Some(text.to_owned());
        } else {
            // Structured JSON and plain text results remain equally inspectable by steps.
            self.last_result = Some(serde_json::from_str(text).unwrap_or_else(|_| json!(text)));
        }
    }

    fn result_text(&self) -> String {
        match &self.last_result {
            Some(v) => serde_json::to_string(v).unwrap_or_default(),
            None => String::new(),
        }
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

// ─── Given Steps ────────────────────────────────────────────────

#[given("a temporary directory")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_temp_dir(w: &mut McpWorld) {
    w.temp_dir = Some(TempDir::new().expect("create temp dir"));
}

#[given(expr = "a file {string} with content {string}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.temp_path().join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    let unescaped = content.replace("\\n", "\n");
    fs::write(&path, &unescaped).unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

#[given(expr = "a KeePass database {string} protected by {string}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_keepass_database(w: &mut McpWorld, name: String, password: String) {
    let mut database = Database::new();
    if let KdfConfig::Argon2 {
        iterations,
        memory,
        parallelism,
        ..
    } = &mut database.config.kdf_config
    {
        *iterations = 1;
        *parallelism = 1;
        *memory = 64 * 1024;
    }
    database.meta.database_name = Some("BDD Vault".to_owned());
    {
        let mut root = database.root_mut();
        let mut entry = root.add_entry();
        entry.set_unprotected(fields::TITLE, "Deploy");
        entry.set_unprotected(fields::USERNAME, "builder");
        entry.set_protected(fields::PASSWORD, "bdd-secret");
        entry.set_unprotected("Custom-Token", "custom-value");
    }

    let path = w.temp_path().join(name);
    let mut file = fs::File::create(path).expect("create KeePass fixture");
    database
        .save(&mut file, DatabaseKey::new().with_password(&password))
        .expect("save KeePass fixture");
}

#[given(expr = "a subdirectory {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_subdir(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    fs::create_dir_all(&path).unwrap_or_else(|e| panic!("mkdir {}: {}", path.display(), e));
}

#[given(expr = "a symlink {string} pointing to {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.temp_path().join(&link);
    let target_path = w.temp_path().join(&target);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    #[cfg(windows)]
    {
        let result = if target_path.is_dir() {
            std::os::windows::fs::symlink_dir(&target_path, &link_path)
        } else {
            std::os::windows::fs::symlink_file(&target_path, &link_path)
        };
        result.unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    }
}

#[given(expr = "a file {string} with content {string} outside the root")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_outside_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.outside_path().join(&name);
    fs::write(&path, content.replace("\\n", "\n"))
        .unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

/// A link *inside* the root naming a file outside it — what a caller plants
/// when the confinement stops at the directory a walk starts from.
#[given(expr = "a symlink {string} pointing outside the root at {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_outside_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.temp_path().join(&link);
    let target_path = w.outside_path().join(&target);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
}

#[given(expr = "the file {string} has mode {string}")]
#[cfg_attr(
    unix,
    expect(
        clippy::expect_used,
        clippy::panic,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )
)]
async fn given_mode(w: &mut McpWorld, name: String, mode: String) {
    let path = w.temp_path().join(&name);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bits = u32::from_str_radix(&mode, 8).expect("octal mode");
        fs::set_permissions(&path, fs::Permissions::from_mode(bits))
            .unwrap_or_else(|e| panic!("chmod {}: {}", path.display(), e));
    }
    #[cfg(not(unix))]
    drop((path, mode));
}

// ─── When Steps ─────────────────────────────────────────────────

#[when(expr = "I export KeePass database {string} with password {string}")]
async fn when_export_keepass_default(w: &mut McpWorld, name: String, password: String) {
    let path = w.resolve(&name);
    w.call_tool("kbdx", json!({"path": path, "password": password}));
}

#[when(expr = "I export KeePass database {string} with password {string} as {string}")]
async fn when_export_keepass_as(w: &mut McpWorld, name: String, password: String, format: String) {
    let path = w.resolve(&name);
    w.call_tool(
        "kbdx",
        json!({"path": path, "password": password, "format": format}),
    );
}

#[when(expr = "I create file {string} with content {string}")]
async fn when_create_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.resolve(&name);
    w.call_tool("create_file", json!({"path": path, "content": content}));
}

#[when(expr = "I try to create file {string} with content {string}")]
async fn when_try_create_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.resolve(&name);
    w.call_tool("create_file", json!({"path": path, "content": content}));
}

#[when(expr = "I write {string} to file {string}")]
async fn when_write_file(w: &mut McpWorld, content: String, name: String) {
    let path = w.resolve(&name);
    w.call_tool("write_file", json!({"path": path, "content": content}));
}

#[when(expr = "I view file {string}")]
async fn when_view_file(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("view_file", json!({"path": path}));
}

#[when(expr = "I view file {string} with offset {int} and limit {int}")]
async fn when_view_file_offset(w: &mut McpWorld, name: String, offset: u64, limit: u64) {
    let path = w.resolve(&name);
    w.call_tool(
        "view_file",
        json!({"path": path, "offset": offset, "limit": limit}),
    );
}

#[when("I try to view the temporary directory")]
async fn when_view_dir(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("view_file", json!({"path": path}));
}

#[when(expr = "I delete {string}")]
async fn when_delete(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("delete", json!({"path": path}));
}

#[when(expr = "I try to delete {string}")]
async fn when_try_delete(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("delete", json!({"path": path}));
}

#[when(expr = "I create directory {string}")]
async fn when_create_dir(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("create_directory", json!({"path": path}));
}

#[when("I list the temporary directory")]
async fn when_list_dir(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("list_directory", json!({"path": path}));
}

#[when(expr = "I try to list {string}")]
async fn when_try_list(w: &mut McpWorld, name: String) {
    if name.is_empty() {
        w.call_tool("list_directory", json!({"path": ""}));
    } else {
        let path = w.resolve(&name);
        w.call_tool("list_directory", json!({"path": path}));
    }
}

#[when("I try to list a path with null byte")]
async fn when_list_null(w: &mut McpWorld) {
    w.call_tool("list_directory", json!({"path": "/tmp/\0bad"}));
}

#[when(expr = "I try to list the absolute path {string}")]
async fn when_try_list_absolute(w: &mut McpWorld, path: String) {
    w.call_tool("list_directory", json!({"path": path}));
}

#[when(expr = "I glob {string}")]
async fn when_glob(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("glob", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I replace {string} with {string}")]
async fn when_replace(w: &mut McpWorld, find: String, replace: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "replace",
        json!({"path": path, "find": find, "replace": replace}),
    );
}

#[when(expr = "I clear the readonly flag on {string}")]
async fn when_clear_readonly(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("permissions", json!({"path": path, "readonly": false}));
}

#[when(expr = "I get disk usage of the temporary directory")]
async fn when_disk_usage(w: &mut McpWorld) {
    let path = w.temp_path().display().to_string();
    w.call_tool("disk_usage", json!({"path": path}));
}

#[when(expr = "I get tree of the temporary directory with depth {int}")]
async fn when_tree(w: &mut McpWorld, depth: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool("tree", json!({"path": path, "depth": depth}));
}

#[when(expr = "I copy {string} to {string}")]
async fn when_copy(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I copy directory {string} to {string}")]
async fn when_copy_dir(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I move {string} to {string}")]
async fn when_move(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool(
        "move_rename",
        json!({"source": source, "destination": dest}),
    );
}

#[when(expr = "I try to copy {string} to {string}")]
async fn when_try_copy(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool("copy", json!({"source": source, "destination": dest}));
}

#[when(expr = "I try to move {string} to {string}")]
async fn when_try_move(w: &mut McpWorld, source: String, dest: String) {
    let source = w.resolve(&source);
    let dest = w.resolve(&dest);
    w.call_tool(
        "move_rename",
        json!({"source": source, "destination": dest}),
    );
}

#[when(expr = "I search for {string}")]
async fn when_search(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("search", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I search for {string} with max results {int}")]
async fn when_search_max(w: &mut McpWorld, pattern: String, max: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "max_results": max}),
    );
}

#[when(expr = "I search for {string} files only")]
async fn when_search_files(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "files_only": true}),
    );
}

#[when(expr = "I search for {string} dirs only")]
async fn when_search_dirs(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "search",
        json!({"path": path, "pattern": pattern, "dirs_only": true}),
    );
}

#[when(expr = "I get info for {string}")]
async fn when_get_info(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("get_file_info", json!({"path": path}));
}

#[when(expr = "I try to get info for {string}")]
async fn when_try_get_info(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("get_file_info", json!({"path": path}));
}

#[when("I list drives")]
async fn when_list_drives(w: &mut McpWorld) {
    w.call_tool("list_drives", json!({}));
}

#[when(expr = "I append {string} to file {string}")]
async fn when_append_file(w: &mut McpWorld, content: String, name: String) {
    let path = w.resolve(&name);
    w.call_tool("append_file", json!({"path": path, "content": content}));
}

#[when(expr = "I checksum {string}")]
async fn when_checksum(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("file_checksum", json!({"path": path}));
}

#[when(expr = "I try to checksum {string}")]
async fn when_try_checksum(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("file_checksum", json!({"path": path}));
}

#[when(expr = "I format {int} bytes")]
async fn when_format_size(w: &mut McpWorld, bytes: u64) {
    w.call_tool("format_size_tool", json!({"bytes": bytes}));
}

#[when(expr = "I ask for a unique name for {string}")]
async fn when_unique_name(w: &mut McpWorld, filename: String) {
    let directory = w.temp_path().display().to_string();
    w.call_tool(
        "unique_name",
        json!({"directory": directory, "filename": filename}),
    );
}

#[when(expr = "I link {string} to {string}")]
async fn when_create_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.resolve(&link);
    let target_path = w.resolve(&target);
    w.call_tool(
        "create_symlink",
        json!({"link": link_path, "target": target_path}),
    );
}

#[when(expr = "I read the link {string}")]
async fn when_read_symlink(w: &mut McpWorld, name: String) {
    let path = w.resolve(&name);
    w.call_tool("read_symlink", json!({"path": path}));
}

/// Comma-separated names, resolved against the scenario's temp directory.
fn resolve_list(w: &McpWorld, names: &str) -> Vec<String> {
    names.split(',').map(|n| w.resolve(n.trim())).collect()
}

#[when(expr = "I batch copy {string} into {string}")]
async fn when_batch_copy(w: &mut McpWorld, names: String, dest: String) {
    let paths = resolve_list(w, &names);
    let destination = w.resolve(&dest);
    w.call_tool(
        "batch_copy",
        json!({"paths": paths, "destination": destination}),
    );
}

#[when(expr = "I batch move {string} into {string}")]
async fn when_batch_move(w: &mut McpWorld, names: String, dest: String) {
    let paths = resolve_list(w, &names);
    let destination = w.resolve(&dest);
    w.call_tool(
        "batch_move",
        json!({"paths": paths, "destination": destination}),
    );
}

#[when(expr = "I batch delete {string}")]
async fn when_batch_delete(w: &mut McpWorld, names: String) {
    let paths = resolve_list(w, &names);
    w.call_tool("batch_delete", json!({"paths": paths}));
}

#[when(expr = "I sort the temporary directory by {string}")]
async fn when_sort_directory(w: &mut McpWorld, sort: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("sort_directory", json!({"path": path, "sort": sort}));
}

#[when(expr = "I sort the file {string} by {string}")]
async fn when_sort_file(w: &mut McpWorld, name: String, sort: String) {
    let path = w.resolve(&name);
    w.call_tool("sort_directory", json!({"path": path, "sort": sort}));
}

#[when(expr = "I grep for {string}")]
async fn when_grep(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("grep", json!({"path": path, "pattern": pattern}));
}

#[when(expr = "I grep for {string} in {string} files")]
async fn when_grep_ext(w: &mut McpWorld, pattern: String, extension: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "grep",
        json!({"path": path, "pattern": pattern, "extension": extension}),
    );
}

#[when(expr = "I find entries named {string}")]
async fn when_find_named(w: &mut McpWorld, name: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("find", json!({"path": path, "name": name}));
}

#[when(expr = "I find entries of type {string}")]
async fn when_find_type(w: &mut McpWorld, file_type: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("find", json!({"path": path, "file_type": file_type}));
}

#[when(expr = "I find entries of at least {int} bytes")]
async fn when_find_min_size(w: &mut McpWorld, min_size: u64) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "find",
        json!({"path": path, "file_type": "file", "min_size": min_size}),
    );
}

#[when(expr = "I find entries no deeper than {int}")]
async fn when_find_max_depth(w: &mut McpWorld, max_depth: usize) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "find",
        json!({"path": path, "file_type": "file", "max_depth": max_depth}),
    );
}

#[when(expr = "I delete files matching {string}")]
async fn when_delete_pattern(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "delete_by_pattern",
        json!({"path": path, "pattern": pattern}),
    );
}

#[when(expr = "I preview deleting files matching {string}")]
async fn when_preview_delete_pattern(w: &mut McpWorld, pattern: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "delete_by_pattern",
        json!({"path": path, "pattern": pattern, "dry_run": true}),
    );
}

#[when(expr = "I preview replacing {string} with {string}")]
async fn when_preview_replace(w: &mut McpWorld, find: String, replace: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool(
        "replace",
        json!({"path": path, "find": find, "replace": replace, "dry_run": true}),
    );
}

#[when(expr = "I bookmark the temporary directory as {string}")]
async fn when_set_bookmark(w: &mut McpWorld, name: String) {
    let path = w.temp_path().display().to_string();
    w.call_tool("set_bookmark", json!({"name": name, "path": path}));
}

#[when(expr = "I look up the bookmark {string}")]
async fn when_get_bookmark(w: &mut McpWorld, name: String) {
    w.call_tool("get_bookmark", json!({"name": name}));
}

#[when("I list bookmarks")]
async fn when_list_bookmarks(w: &mut McpWorld) {
    w.call_tool("list_bookmarks", json!({}));
}

#[when(expr = "I remove the bookmark {string}")]
async fn when_delete_bookmark(w: &mut McpWorld, name: String) {
    w.call_tool("delete_bookmark", json!({"name": name}));
}

#[when("I get system info")]
async fn when_system_info(w: &mut McpWorld) {
    w.call_tool("system_info", json!({}));
}

#[when("I list AWS profiles")]
async fn when_list_aws_profiles(w: &mut McpWorld) {
    w.call_tool("s3_list_profiles", json!({}));
}

// ─── Then Steps ─────────────────────────────────────────────────

#[then(expr = "the file {string} should exist")]
async fn then_file_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(path.exists(), "file should exist: {}", path.display());
}

#[then(expr = "the file {string} should not exist")]
async fn then_file_not_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(!path.exists(), "file should not exist: {}", path.display());
}

#[then(expr = "the file {string} should contain {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_file_contains(w: &mut McpWorld, name: String, expected: String) {
    let path = w.temp_path().join(&name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(
        content.contains(&expected),
        "file '{name}' should contain '{expected}', got '{content}'"
    );
}

#[then(expr = "the file {string} should not contain {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_file_not_contains(w: &mut McpWorld, name: String, unexpected: String) {
    let path = w.temp_path().join(&name);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(
        !content.contains(&unexpected),
        "file '{name}' should NOT contain '{unexpected}', got '{content}'"
    );
}

#[then(expr = "the symlink {string} should not exist")]
async fn then_symlink_not_exists(w: &mut McpWorld, name: String) {
    // `exists()` follows the link, so it cannot tell a deleted link from a
    // dangling one — ask for the link's own metadata instead.
    let path = w.temp_path().join(&name);
    assert!(
        fs::symlink_metadata(&path).is_err(),
        "symlink should be gone: {}",
        path.display()
    );
}

#[then(expr = "{string} should still be a symlink")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_still_a_symlink(w: &mut McpWorld, name: String) {
    // `exists()` follows the link and would pass even after the link had been
    // renamed away and replaced by a regular file — ask for its own metadata.
    let path = w.temp_path().join(&name);
    let meta = fs::symlink_metadata(&path)
        .unwrap_or_else(|e| panic!("symlink_metadata {}: {}", path.display(), e));
    assert!(
        meta.file_type().is_symlink(),
        "{} should still be a symlink",
        path.display()
    );
}

#[then(expr = "the result field {string} should be {int}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_result_field_is(w: &mut McpWorld, field: String, expected: u64) {
    let result = w.last_result.as_ref().expect("no result");
    let actual = result[&field]
        .as_u64()
        .unwrap_or_else(|| panic!("no numeric field '{}' in {}", field, w.result_text()));
    assert_eq!(actual, expected, "field '{field}' mismatch");
}

#[then(expr = "the directory {string} should exist")]
async fn then_dir_exists(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    assert!(path.is_dir(), "directory should exist: {}", path.display());
}

#[then(expr = "the result should contain {string}")]
async fn then_result_contains(w: &mut McpWorld, expected: String) {
    let text = w.result_text();
    assert!(
        text.contains(&expected),
        "result should contain '{}', got: {}",
        expected,
        preview(&text)
    );
}

#[then(expr = "the result should not contain {string}")]
async fn then_result_not_contains(w: &mut McpWorld, expected: String) {
    let text = w.result_text();
    assert!(
        !text.contains(&expected),
        "result should NOT contain '{}', got: {}",
        expected,
        preview(&text)
    );
}

/// The start of `text` for a failure message: its first 200 bytes, cut back
/// to a character boundary so a multibyte character (`├──`) is never split.
fn preview(text: &str) -> &str {
    let (start, _) = text.split_at(text.floor_char_boundary(200));
    start
}

#[then(expr = "the operation should fail with {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_error_contains(w: &mut McpWorld, expected: String) {
    let err = w.last_error.as_ref().unwrap_or_else(|| {
        panic!("expected error containing '{expected}', but operation succeeded")
    });
    assert!(
        err.to_lowercase().contains(&expected.to_lowercase()),
        "error should contain '{expected}', got: {err}"
    );
}

#[then("the operation should fail")]
async fn then_error(w: &mut McpWorld) {
    assert!(
        w.last_error.is_some(),
        "expected an error, but operation succeeded with: {}",
        w.result_text()
    );
}

#[then(expr = "the search should return {int} results")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_search_count(w: &mut McpWorld, count: usize) {
    let result = w.last_result.as_ref().expect("no result");
    let actual = usize::try_from(result["count"].as_u64().expect("no count field"))
        .expect("count fits in usize");
    assert_eq!(
        actual, count,
        "expected {count} search results, got {actual}"
    );
}

#[then(expr = "the size should be {int}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_size_is(w: &mut McpWorld, expected: u64) {
    let result = w.last_result.as_ref().expect("no result");
    let actual = result["size"].as_u64().expect("no size field");
    assert_eq!(actual, expected, "expected size {expected}, got {actual}");
}

#[then(expr = "the result field {string} should equal {string}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_result_field_equals(w: &mut McpWorld, field: String, expected: String) {
    let result = w.last_result.as_ref().expect("no result");
    let actual = result[&field]
        .as_str()
        .unwrap_or_else(|| panic!("no string field '{}' in {}", field, w.result_text()));
    assert_eq!(actual, expected, "field '{field}' mismatch");
}

/// The `name` of every object in the array under `field`.
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
fn entry_names<'a>(result: &'a Value, field: &str) -> Vec<&'a str> {
    result[field]
        .as_array()
        .unwrap_or_else(|| panic!("no array field '{field}'"))
        .iter()
        .filter_map(|e| e["name"].as_str())
        .collect()
}

fn expected_names(expected: &str) -> Vec<&str> {
    expected.split(',').map(str::trim).collect()
}

/// Ordered comparison — for listings where the order *is* the behavior.
#[then(expr = "the {string} entries should be named {string}")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_named_in_order(w: &mut McpWorld, field: String, expected: String) {
    let result = w.last_result.as_ref().expect("no result");
    assert_eq!(
        entry_names(result, &field),
        expected_names(&expected),
        "order of '{field}' mismatch"
    );
}

/// Unordered comparison — directory iteration order is not defined, so for a
/// recursive walk only the set of entries is meaningful.
#[then(expr = "the {string} entries should be named {string} in any order")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_named_any_order(w: &mut McpWorld, field: String, expected: String) {
    let result = w.last_result.as_ref().expect("no result");
    let mut names = entry_names(result, &field);
    names.sort_unstable();
    let mut want = expected_names(&expected);
    want.sort_unstable();
    assert_eq!(names, want, "contents of '{field}' mismatch");
}

/// Compares a reported path against the scenario's temp directory. The server
/// canonicalizes some paths and echoes others verbatim, so both forms pass.
#[then(expr = "the result field {string} should be the temporary directory")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_field_is_temp_dir(w: &mut McpWorld, field: String) {
    let temp = w.temp_path();
    let canonical = temp.canonicalize().unwrap_or_else(|_| temp.clone());
    let text = w.result_text();
    let result = w.last_result.as_ref().expect("no result");
    let actual = result[&field]
        .as_str()
        .unwrap_or_else(|| panic!("no string field '{field}' in {text}"));
    assert!(
        Path::new(actual) == temp || Path::new(actual) == canonical,
        "field '{}' is '{}', expected the temp dir '{}'",
        field,
        actual,
        temp.display()
    );
}

#[then(expr = "the result flag {string} should be true")]
async fn then_flag_true(w: &mut McpWorld, field: String) {
    assert_flag(w, &field, true);
}

#[then(expr = "the result flag {string} should be false")]
async fn then_flag_false(w: &mut McpWorld, field: String) {
    assert_flag(w, &field, false);
}

#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
fn assert_flag(w: &McpWorld, field: &str, expected: bool) {
    let text = w.result_text();
    let result = w.last_result.as_ref().expect("no result");
    let actual = result[field]
        .as_bool()
        .unwrap_or_else(|| panic!("no boolean field '{field}' in {text}"));
    assert_eq!(actual, expected, "flag '{field}' mismatch");
}

#[then(expr = "profile {int} should be {string} in region {string}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_profile_region(w: &mut McpWorld, index: usize, name: String, region: String) {
    let text = w.result_text();
    let result = w.last_result.as_ref().expect("no result");
    let profiles = result["profiles"].as_array().expect("no profiles array");
    let profile = profiles
        .get(index - 1)
        .unwrap_or_else(|| panic!("no profile #{index} in {text}"));
    assert_eq!(profile["name"].as_str(), Some(name.as_str()));
    assert_eq!(
        profile["region"].as_str(),
        Some(region.as_str()),
        "region attached to the wrong profile in {text}"
    );
}

/// A list of plain strings under `field`, in the order the server reported it.
#[then(expr = "the result list {string} should be {string}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_result_list_is(w: &mut McpWorld, field: String, expected: String) {
    let result = w.last_result.as_ref().expect("no result");
    let items = result[&field]
        .as_array()
        .unwrap_or_else(|| panic!("no array field '{}' in {}", field, w.result_text()));
    let names: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
    let want: Vec<&str> = expected.split(',').map(str::trim).collect();
    assert_eq!(names, want, "list '{field}' mismatch");
}

#[then(expr = "match {int} should be {string} at line {int}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_grep_match(w: &mut McpWorld, index: usize, content: String, line: u64) {
    let result = w.last_result.as_ref().expect("no result");
    let matches = result["matches"].as_array().expect("no matches array");
    let hit = matches
        .get(index - 1)
        .unwrap_or_else(|| panic!("no match #{} in {}", index, w.result_text()));
    assert_eq!(hit["content"].as_str(), Some(content.as_str()));
    assert_eq!(hit["line"].as_u64(), Some(line), "line number mismatch");
}

#[then("the server should exit cleanly after its input closes")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_server_exits_cleanly(w: &mut McpWorld) {
    // A server that has to be killed reports a signal here instead of code 0.
    // That distinction is the difference between coverage being recorded and
    // being silently thrown away, so one scenario asserts on it directly.
    let status = w.server_exit.expect("no server process was started");
    assert!(
        status.success(),
        "server should exit 0 once stdin closes, got {status:?}"
    );
}

#[then(expr = "{string} should appear before {string} in entries")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn then_order(w: &mut McpWorld, first: String, second: String) {
    let result = w.last_result.as_ref().expect("no result");
    let entries = result["entries"].as_array().expect("no entries array");
    let names: Vec<&str> = entries.iter().filter_map(|e| e["name"].as_str()).collect();
    let pos_first = names.iter().position(|n| *n == first);
    let pos_second = names.iter().position(|n| *n == second);
    assert!(
        pos_first < pos_second,
        "'{first}' (pos {pos_first:?}) should appear before '{second}' (pos {pos_second:?}) in {names:?}"
    );
}

// ─── Main ───────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    McpWorld::filter_run("tests/features", |feature, rule, scenario| {
        cfg!(unix)
            || !feature
                .tags
                .iter()
                .chain(rule.into_iter().flat_map(|rule| &rule.tags))
                .chain(scenario.tags.iter())
                .any(|tag| tag == "unix")
    })
    .await;
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
