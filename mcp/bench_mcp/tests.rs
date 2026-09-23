use super::*;

#[test]
fn a_cargo_example_uses_its_shared_or_custom_artifact_directory() {
    let temp = tempfile::TempDir::new().expect("temporary target");
    let target = temp.path().join("custom target");
    let executable = target.join("debug/examples/bench_mcp");
    let candidates = server_candidates(&executable);
    let name = format!("ffm-mcp{}", std::env::consts::EXE_SUFFIX);
    assert_eq!(candidates[0], target.join("release").join(&name));
    assert_eq!(candidates[1], target.join("debug").join(name));
}

#[test]
fn standalone_compilation_retains_the_local_target_fallback() {
    let temp = tempfile::TempDir::new().expect("temporary executable directory");
    let executable = temp.path().join("bench_mcp");
    let candidates = server_candidates(&executable);
    let name = format!("ffm-mcp{}", std::env::consts::EXE_SUFFIX);
    assert_eq!(candidates[0], Path::new("target/release").join(&name));
    assert_eq!(candidates[1], Path::new("target/debug").join(name));
}

const TOOL_ORDER: [&str; 35] = [
    "list_directory",
    "sort_directory",
    "get_file_info",
    "view_file",
    "create_file",
    "write_file",
    "append_file",
    "create_directory",
    "copy",
    "move_rename",
    "delete",
    "search",
    "grep",
    "tree",
    "glob",
    "find",
    "replace",
    "delete_by_pattern",
    "list_drives",
    "disk_usage",
    "unique_name",
    "file_checksum",
    "format_size_tool",
    "batch_copy",
    "batch_move",
    "batch_delete",
    "create_symlink",
    "read_symlink",
    "permissions",
    "set_bookmark",
    "get_bookmark",
    "list_bookmarks",
    "delete_bookmark",
    "system_info",
    "s3_list_profiles",
];

#[test]
fn benchmark_groups_keep_all_requests_ordered_and_report_mock_failures() {
    let temp = tempfile::TempDir::new().expect("isolated benchmark fixtures");
    create_fixtures(temp.path());
    assert_eq!(
        std::fs::read_to_string(temp.path().join("subdir/b.rs")).expect("fixture"),
        "fn main() {}"
    );
    let responses = (0..TOOL_ORDER.len())
        .map(|index| {
            let response = match index {
                5 => serde_json::json!({"result": {"isError": true}}),
                9 => serde_json::json!({"error": {"message": "mock failure"}}),
                _ => serde_json::json!({"result": {"isError": false}}),
            };
            format!("{response}\n")
        })
        .collect::<String>();
    let mut reader = std::io::Cursor::new(responses);
    let mut requests = Vec::new();
    let results = benchmark_tools(temp.path(), &mut requests, &mut reader);
    assert_eq!(
        results.iter().map(|(name, _, _)| *name).collect::<Vec<_>>(),
        TOOL_ORDER
    );
    assert_eq!(results.iter().filter(|(_, _, success)| !success).count(), 2);
    assert!(!results[5].2);
    assert!(!results[9].2);

    let requests = String::from_utf8(requests).expect("UTF-8 protocol");
    let requests: Vec<serde_json::Value> = requests
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid JSON even for Windows paths"))
        .collect();
    assert_eq!(requests.len(), TOOL_ORDER.len());
    for (index, request) in requests.iter().enumerate() {
        assert_eq!(request["params"]["name"], TOOL_ORDER[index]);
        assert_eq!(request["id"], u64::try_from(index + 2).expect("request id"));
    }
    assert_eq!(
        requests[0]["params"]["arguments"]["path"],
        temp.path().to_string_lossy().as_ref()
    );
    assert_eq!(requests[6]["params"]["arguments"]["content"], "\nappended");
    assert!(
        temp.path().join("delete_me.txt").exists(),
        "mock calls must not execute file operations"
    );
}

#[test]
fn benchmark_json_paths_escape_separators_quotes_and_control_characters() {
    let path = Path::new("C:\\quoted \"path\"\\line\nwith\ttabs");
    let decoded: String =
        serde_json::from_str(&format!("\"{}\"", json_path(path))).expect("JSON path");
    assert_eq!(decoded, path.to_string_lossy());
}

/// This file compiled on its own, as its header documents, into `directory`,
/// and a command that runs it there. No Cargo target sits beside it, so it
/// finds no server by accident, and its scratch directory stays inside.
fn standalone_benchmark(directory: &Path) -> Command {
    let binary = directory.join(format!("bench_mcp{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition", "2021"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("bench_mcp.rs"))
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("run rustc");
    assert!(
        compiled.status.success(),
        "the benchmark compiles on its own: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let mut benchmark = Command::new(binary);
    benchmark
        .current_dir(directory)
        .env_remove("FFM_MCP_BIN")
        .env("RUST_BACKTRACE", "0");
    for variable in ["TMPDIR", "TMP", "TEMP"] {
        benchmark.env(variable, directory);
    }
    benchmark
}

#[test]
fn without_a_server_the_benchmark_prints_one_error_line_and_exits_with_1() {
    let temp = tempfile::TempDir::new().expect("empty directory");
    let output = standalone_benchmark(temp.path())
        .output()
        .expect("run the benchmark");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "ERROR: ffm-mcp binary not found. Run 'cargo build --release' first or set FFM_MCP_BIN.\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn a_server_that_cannot_start_aborts_the_benchmark() {
    let temp = tempfile::TempDir::new().expect("empty directory");
    let output = standalone_benchmark(temp.path())
        .env("FFM_MCP_BIN", temp.path().join("no-such-server"))
        .output()
        .expect("run the benchmark");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(101), "{stderr}");
    assert!(stderr.contains("failed to start ffm-mcp: Os {"), "{stderr}");
}

#[test]
fn a_scratch_directory_that_cannot_be_made_aborts_the_benchmark() {
    let temp = tempfile::TempDir::new().expect("empty directory");
    let not_a_directory = temp.path().join("a file");
    std::fs::write(&not_a_directory, "").expect("plain file");
    let mut benchmark = standalone_benchmark(temp.path());
    for variable in ["TMPDIR", "TMP", "TEMP"] {
        benchmark.env(variable, &not_a_directory);
    }
    let output = benchmark
        .env("FFM_MCP_BIN", temp.path().join("no-such-server"))
        .output()
        .expect("run the benchmark");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(101), "{stderr}");
    assert!(
        stderr.contains("called `Result::unwrap()` on an `Err` value: Os {"),
        "{stderr}"
    );
}
