#!/usr/bin/env rust-script
//! Benchmark all FFM MCP server tools via JSON-RPC over stdio.
//!
//! Usage:
//!   `cargo build --release -j1 && cargo run --example bench_mcp`
//! Or standalone:
//!   `rustc --edition 2021 bench_mcp.rs -o bench_mcp && ./bench_mcp`

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
#[expect(
    clippy::print_stderr,
    reason = "a person who runs the benchmark reads why it cannot start"
)]
fn main() -> ExitCode {
    let binary = match locate_server() {
        Ok(binary) => binary,
        Err(message) => {
            eprintln!("ERROR: {message}");
            return ExitCode::FAILURE;
        }
    };

    println!("=== FFM MCP Server Benchmark ===");
    println!("Binary: {}", binary.display());
    println!();

    // Create temp directory for test files. A previous run may have left one
    // behind, or none: either way the directory is recreated empty.
    let tmp = std::env::temp_dir().join("ffm_mcp_bench");
    drop(std::fs::remove_dir_all(&tmp));
    create_fixtures(&tmp);

    let (mut child, mut stdin, stdout) = start_server(&binary, &tmp);
    let mut reader = BufReader::new(stdout);

    // Send initialize
    let init_req = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bench","version":"1.0"}}}"#;
    send(&mut stdin, init_req);
    recv(&mut reader); // initialize response

    // Send initialized notification
    let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send(&mut stdin, init_notif);

    // Small delay for server to be ready
    std::thread::sleep(Duration::from_millis(50));

    let results = benchmark_tools(&tmp, &mut stdin, &mut reader);
    print_results(&results);

    // Cleanup is best effort: the results are already reported, and a
    // leftover fixture directory is removed by the next run.
    drop(child.kill());
    // `kill` only sends the signal; without the wait the server stays a zombie
    // until this process exits.
    drop(child.wait());
    drop(std::fs::remove_dir_all(&tmp));
    ExitCode::SUCCESS
}

/// Start the server confined to `root`, with its stdin and stdout piped to us.
#[expect(
    clippy::expect_used,
    reason = "a benchmark without its server has nothing to measure: abort"
)]
#[expect(
    clippy::unwrap_used,
    reason = "both streams are `Stdio::piped()`, so the child always has them"
)]
fn start_server(binary: &Path, root: &Path) -> (Child, ChildStdin, ChildStdout) {
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
    reason = "without its fixtures the benchmark has nothing to measure: abort"
)]
fn create_fixtures(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    for directory in [
        "subdir",
        "copy_dest",
        "delete_dir",
        "batch_dest",
        "bmove_dest",
        "replace_dir",
        "pattern_dir",
    ] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    create_fixture_files(root).unwrap();
}

fn benchmark_tools(
    root: &Path,
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
) -> Vec<(&'static str, Duration, bool)> {
    let mut results = Vec::new();
    let mut request_id = 1_u64;
    let mut bench = |name: &'static str, arguments: String| {
        request_id += 1;
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":{request_id},"method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"#
        );
        let start = Instant::now();
        send(stdin, &request);
        let response = recv(reader);
        let elapsed = start.elapsed();
        let ok = !response.contains("\"isError\":true") && !response.contains("\"error\":");
        results.push((name, elapsed, ok));
    };
    file_operations(&mut bench, root);
    search_operations(&mut bench, root);
    metadata_operations(&mut bench, root);
    batch_operations(&mut bench, root);
    system_operations(&mut bench, root);
    results
}

/// Escape a path for insertion inside a JSON string, keeping standalone rustc support.
#[expect(
    clippy::let_underscore_must_use,
    reason = "writing into a `String` cannot fail"
)]
fn json_path(path: &Path) -> String {
    let mut escaped = String::new();
    for character in path.to_string_lossy().chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            control if control.is_control() => {
                // Writing into a `String` cannot fail.
                let _ = write!(escaped, "\\u{:04x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

fn file_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_file_str = json_path(&tmp.join("test.txt"));
    let test_dir_str = json_path(&tmp.join("subdir"));

    // ── 1. list_directory ────────────────────────────────────────
    bench("list_directory", format!(r#"{{"path":"{tmp_str}"}}"#));

    // ── 2. sort_directory ────────────────────────────────────────
    bench(
        "sort_directory",
        format!(r#"{{"path":"{test_dir_str}","sort":"size"}}"#),
    );

    // ── 3. get_file_info ─────────────────────────────────────────
    bench("get_file_info", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 4. view_file ─────────────────────────────────────────────
    bench("view_file", format!(r#"{{"path":"{test_file_str}"}}"#));

    write_operations(bench, tmp);
}

fn search_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_dir_str = json_path(&tmp.join("subdir"));

    // ── 12. search ───────────────────────────────────────────────
    bench(
        "search",
        format!(r#"{{"path":"{tmp_str}","pattern":"txt"}}"#),
    );

    // ── 13. grep ─────────────────────────────────────────────────
    bench(
        "grep",
        format!(r#"{{"path":"{test_dir_str}","pattern":"content"}}"#),
    );

    // ── 14. tree ─────────────────────────────────────────────────
    bench("tree", format!(r#"{{"path":"{tmp_str}","depth":2}}"#));

    // ── 15. glob ─────────────────────────────────────────────────
    bench(
        "glob",
        format!(r#"{{"path":"{tmp_str}","pattern":"*.txt"}}"#),
    );

    // ── 16. find ─────────────────────────────────────────────────
    bench(
        "find",
        format!(r#"{{"path":"{tmp_str}","name":"*.txt","file_type":"file"}}"#),
    );

    replace_and_delete_operations(bench, tmp);
}

fn metadata_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_dir_str = json_path(&tmp.join("subdir"));
    let test_file_str = json_path(&tmp.join("test.txt"));

    // ── 19. list_drives ──────────────────────────────────────────
    bench("list_drives", "{}".to_owned());

    // ── 20. disk_usage ───────────────────────────────────────────
    bench("disk_usage", format!(r#"{{"path":"{test_dir_str}"}}"#));

    // ── 21. unique_name ──────────────────────────────────────────
    bench(
        "unique_name",
        format!(r#"{{"directory":"{tmp_str}","filename":"test.txt"}}"#),
    );

    // ── 22. file_checksum ────────────────────────────────────────
    bench("file_checksum", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 23. format_size_tool ───────────────────────────────────────
    bench("format_size_tool", r#"{"bytes":1048576}"#.to_owned());
}

fn batch_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let batch1 = tmp.join("batch1.txt");
    let batch2 = tmp.join("batch2.txt");
    let batch_dest = tmp.join("batch_dest");
    let bmove1 = tmp.join("bmove1.txt");
    let bmove2 = tmp.join("bmove2.txt");
    let bmove_dest = tmp.join("bmove_dest");

    // ── 24. batch_copy ───────────────────────────────────────────
    bench(
        "batch_copy",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            json_path(&batch1),
            json_path(&batch2),
            json_path(&batch_dest)
        ),
    );

    // ── 25. batch_move ───────────────────────────────────────────
    bench(
        "batch_move",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            json_path(&bmove1),
            json_path(&bmove2),
            json_path(&bmove_dest)
        ),
    );

    batch_delete_operations(bench, tmp);
}

fn system_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_file_str = json_path(&tmp.join("test.txt"));

    // ── 29. permissions ──────────────────────────────────────────
    bench("permissions", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 30. set_bookmark ─────────────────────────────────────────
    bench(
        "set_bookmark",
        format!(r#"{{"name":"bench_test","path":"{tmp_str}"}}"#),
    );

    // ── 31. get_bookmark ─────────────────────────────────────────
    bench("get_bookmark", r#"{"name":"bench_test"}"#.to_owned());

    // ── 32. list_bookmarks ───────────────────────────────────────
    bench("list_bookmarks", "{}".to_owned());

    // ── 33. delete_bookmark ──────────────────────────────────────
    bench("delete_bookmark", r#"{"name":"bench_test"}"#.to_owned());

    // ── 34. system_info ──────────────────────────────────────────
    bench("system_info", "{}".to_owned());

    // ── 35. s3_list_profiles ─────────────────────────────────────
    bench("s3_list_profiles", "{}".to_owned());
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_results(results: &[(&str, Duration, bool)]) {
    // Print results
    println!("┌─────────────────────────┬───────────┬────────┐");
    println!("│ Tool                    │ Time      │ Status │");
    println!("├─────────────────────────┼───────────┼────────┤");

    let mut total = Duration::ZERO;
    let mut pass_count = 0u32;

    for (name, dur, ok) in results {
        let status = if *ok { "  OK  " } else { " FAIL " };
        let time_str = format_duration(*dur);
        println!("│ {name:<23} │ {time_str:>9} │{status}│");
        total += *dur;
        if *ok {
            pass_count += 1;
        }
    }

    print_skipped_tools();

    println!("├─────────────────────────┼───────────┼────────┤");
    println!(
        "│ TOTAL ({} tools)         │ {:>9} │{}/{} ok│",
        results.len(),
        format_duration(total),
        pass_count,
        results.len()
    );
    println!("└─────────────────────────┴───────────┴────────┘");
    println!();
    println!("Skipped: {} S3 tools (need AWS credentials)", 6);

    print_statistics(results, total);
    print_failures(results);
}

#[expect(
    clippy::unwrap_used,
    reason = "a benchmark that has lost its server has nothing left to measure: abort"
)]
fn send(stdin: &mut impl Write, msg: &str) {
    // rmcp uses line-delimited JSON (no Content-Length header)
    stdin.write_all(msg.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "a benchmark that has lost its server has nothing left to measure: abort"
)]
fn recv(reader: &mut impl BufRead) -> String {
    // Read one JSON line
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    line
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 {
        format!("{:.0} µs", ms * 1000.0)
    } else if ms < 1000.0 {
        format!("{ms:.2} ms")
    } else {
        format!("{:.2} s", ms / 1000.0)
    }
}

/// Find server artifacts beside this Cargo example, including custom targets.
/// The cwd-relative fallback also keeps the standalone rustc workflow working.
fn server_candidates(executable: &Path) -> Vec<PathBuf> {
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

#[cfg(test)]
#[path = "bench_mcp/tests.rs"]
mod tests;

fn locate_server() -> Result<PathBuf, &'static str> {
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

fn create_fixture_files(root: &Path) -> std::io::Result<()> {
    for (name, content) in [
        ("test.txt", "Hello, World!\nLine 2\nLine 3\nfoo bar baz\n"),
        ("subdir/a.txt", "content a"),
        ("subdir/b.rs", "fn main() {}"),
        ("subdir/c.tmp", "temporary"),
        ("move_me.txt", "move me"),
        ("delete_me.txt", "delete me"),
        ("delete_dir/x.txt", "x"),
        ("batch1.txt", "batch1"),
        ("batch2.txt", "batch2"),
        ("bmove1.txt", "bmove1"),
        ("bmove2.txt", "bmove2"),
        ("bdel1.txt", "bdel1"),
        ("bdel2.txt", "bdel2"),
        ("symlink_target.txt", "target"),
        ("replace_dir/r1.txt", "old_text here and old_text there"),
        ("replace_dir/r2.txt", "no match here"),
        ("pattern_dir/keep.txt", "keep"),
        ("pattern_dir/a.tmp", "tmp1"),
        ("pattern_dir/b.tmp", "tmp2"),
    ] {
        std::fs::write(root.join(name), content)?;
    }
    Ok(())
}

fn write_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    // ── 5. create_file ───────────────────────────────────────────
    let new_file = tmp.join("created.txt");
    bench(
        "create_file",
        format!(
            r#"{{"path":"{}","content":"bench content"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 6. write_file ────────────────────────────────────────────
    bench(
        "write_file",
        format!(
            r#"{{"path":"{}","content":"overwritten"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 7. append_file ───────────────────────────────────────────
    bench(
        "append_file",
        format!(
            r#"{{"path":"{}","content":"\nappended"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 8. create_directory ──────────────────────────────────────
    let new_dir = tmp.join("bench_newdir");
    bench(
        "create_directory",
        format!(r#"{{"path":"{}"}}"#, json_path(&new_dir)),
    );

    transfer_operations(bench, tmp);
}

fn transfer_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let test_file_str = json_path(&tmp.join("test.txt"));
    let copy_dest = tmp.join("copy_dest");
    let move_src = tmp.join("move_me.txt");
    let delete_file = tmp.join("delete_me.txt");
    // ── 9. copy ──────────────────────────────────────────────────
    bench(
        "copy",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            test_file_str,
            json_path(&copy_dest.join("copied.txt"))
        ),
    );

    // ── 10. move_rename ──────────────────────────────────────────
    bench(
        "move_rename",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            json_path(&move_src),
            json_path(&tmp.join("moved.txt"))
        ),
    );

    // ── 11. delete ───────────────────────────────────────────────
    bench(
        "delete",
        format!(r#"{{"path":"{}"}}"#, json_path(&delete_file)),
    );
}

fn replace_and_delete_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let replace_dir = tmp.join("replace_dir");
    let pat_dir = tmp.join("pattern_dir");
    // ── 17. replace ──────────────────────────────────────────────
    bench(
        "replace",
        format!(
            r#"{{"path":"{}","find":"old_text","replace":"new_text"}}"#,
            json_path(&replace_dir)
        ),
    );

    // ── 18. delete_by_pattern ────────────────────────────────────
    bench(
        "delete_by_pattern",
        format!(r#"{{"path":"{}","pattern":"*.tmp"}}"#, json_path(&pat_dir)),
    );
}

fn symlink_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let symlink_target = tmp.join("symlink_target.txt");
    let symlink_link = tmp.join("test_symlink");
    // ── 27. create_symlink ───────────────────────────────────────
    bench(
        "create_symlink",
        format!(
            r#"{{"link":"{}","target":"{}"}}"#,
            json_path(&symlink_link),
            json_path(&symlink_target)
        ),
    );

    // ── 28. read_symlink ─────────────────────────────────────────
    bench(
        "read_symlink",
        format!(r#"{{"path":"{}"}}"#, json_path(&symlink_link)),
    );
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_statistics(results: &[(&str, Duration, bool)], total: Duration) {
    // No results, no stats.
    let (Some(&(min_name, _, _)), Some(&(max_name, _, _))) = (
        results.iter().min_by_key(|(_, d, _)| *d),
        results.iter().max_by_key(|(_, d, _)| *d),
    ) else {
        return;
    };
    let times: Vec<f64> = results
        .iter()
        .map(|(_, d, _)| d.as_secs_f64() * 1000.0)
        .collect();
    let count = u32::try_from(times.len()).unwrap_or(u32::MAX);
    let avg = times.iter().sum::<f64>() / f64::from(count);
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);
    let max = times.iter().copied().fold(0.0f64, f64::max);
    println!();
    println!("Stats:");
    println!("  Avg: {avg:.2} ms");
    println!("  Min: {min:.2} ms ({min_name})");
    println!("  Max: {max:.2} ms ({max_name})");
    println!("  Total: {:.2} ms", total.as_secs_f64() * 1000.0);
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_failures(results: &[(&str, Duration, bool)]) {
    if results.iter().any(|(_, _, ok)| !ok) {
        println!();
        println!("FAILURES:");
        for (name, _, ok) in results {
            if !ok {
                println!("  - {name}");
            }
        }
    }
}

fn batch_delete_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let bdel1 = tmp.join("bdel1.txt");
    let bdel2 = tmp.join("bdel2.txt");
    // ── 26. batch_delete ─────────────────────────────────────────
    bench(
        "batch_delete",
        format!(
            r#"{{"paths":["{}","{}"]}}"#,
            json_path(&bdel1),
            json_path(&bdel2)
        ),
    );

    symlink_operations(bench, tmp);
}

#[expect(clippy::print_stdout, reason = "the benchmark's report is its output")]
fn print_skipped_tools() {
    // S3 tools that require real AWS credentials — skip with note
    let s3_skipped = [
        "s3_list_buckets",
        "s3_list_objects",
        "s3_download",
        "s3_upload",
        "s3_delete",
        "s3_copy",
    ];

    for name in &s3_skipped {
        println!("│ {name:<23} │   skipped │  S3   │");
    }
}
