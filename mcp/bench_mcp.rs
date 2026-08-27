#!/usr/bin/env rust-script
//! Benchmark all FFM MCP server tools via JSON-RPC over stdio.
//!
//! Usage:
//!   cargo build --release -j1 && cargo run --example bench_mcp
//! Or standalone:
//!   rustc bench_mcp.rs -o bench_mcp && ./bench_mcp

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn main() {
    let binary = std::env::var("FFM_MCP_BIN").unwrap_or_else(|_| {
        // Try release first, then debug
        let release = "target/release/ffm-mcp";
        let debug = "target/debug/ffm-mcp";
        if std::path::Path::new(release).exists() {
            release.to_string()
        } else if std::path::Path::new(debug).exists() {
            debug.to_string()
        } else {
            eprintln!("ERROR: ffm-mcp binary not found. Run 'cargo build --release' first.");
            std::process::exit(1);
        }
    });

    println!("=== FFM MCP Server Benchmark ===");
    println!("Binary: {binary}");
    println!();

    // Create temp directory for test files
    let tmp = std::env::temp_dir().join("ffm_mcp_bench");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let tmp_str = tmp.display().to_string();

    // Create test fixtures
    let test_file = tmp.join("test.txt");
    std::fs::write(&test_file, "Hello, World!\nLine 2\nLine 3\nfoo bar baz\n").unwrap();
    let test_file_str = test_file.display().to_string();

    let test_dir = tmp.join("subdir");
    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("a.txt"), "content a").unwrap();
    std::fs::write(test_dir.join("b.rs"), "fn main() {}").unwrap();
    std::fs::write(test_dir.join("c.tmp"), "temporary").unwrap();
    let test_dir_str = test_dir.display().to_string();

    let copy_dest = tmp.join("copy_dest");
    std::fs::create_dir_all(&copy_dest).unwrap();

    let move_src = tmp.join("move_me.txt");
    std::fs::write(&move_src, "move me").unwrap();

    let delete_file = tmp.join("delete_me.txt");
    std::fs::write(&delete_file, "delete me").unwrap();

    let delete_dir = tmp.join("delete_dir");
    std::fs::create_dir_all(&delete_dir).unwrap();
    std::fs::write(delete_dir.join("x.txt"), "x").unwrap();

    // Files for batch operations
    let batch1 = tmp.join("batch1.txt");
    let batch2 = tmp.join("batch2.txt");
    std::fs::write(&batch1, "batch1").unwrap();
    std::fs::write(&batch2, "batch2").unwrap();
    let batch_dest = tmp.join("batch_dest");
    std::fs::create_dir_all(&batch_dest).unwrap();

    // Files for batch move
    let bmove1 = tmp.join("bmove1.txt");
    let bmove2 = tmp.join("bmove2.txt");
    std::fs::write(&bmove1, "bmove1").unwrap();
    std::fs::write(&bmove2, "bmove2").unwrap();
    let bmove_dest = tmp.join("bmove_dest");
    std::fs::create_dir_all(&bmove_dest).unwrap();

    // Files for batch delete
    let bdel1 = tmp.join("bdel1.txt");
    let bdel2 = tmp.join("bdel2.txt");
    std::fs::write(&bdel1, "bdel1").unwrap();
    std::fs::write(&bdel2, "bdel2").unwrap();

    // Symlink target
    let symlink_target = tmp.join("symlink_target.txt");
    std::fs::write(&symlink_target, "target").unwrap();
    let symlink_link = tmp.join("test_symlink");

    // Replace test files
    let replace_dir = tmp.join("replace_dir");
    std::fs::create_dir_all(&replace_dir).unwrap();
    std::fs::write(
        replace_dir.join("r1.txt"),
        "old_text here and old_text there",
    )
    .unwrap();
    std::fs::write(replace_dir.join("r2.txt"), "no match here").unwrap();

    // Delete-by-pattern test files
    let pat_dir = tmp.join("pattern_dir");
    std::fs::create_dir_all(&pat_dir).unwrap();
    std::fs::write(pat_dir.join("keep.txt"), "keep").unwrap();
    std::fs::write(pat_dir.join("a.tmp"), "tmp1").unwrap();
    std::fs::write(pat_dir.join("b.tmp"), "tmp2").unwrap();

    // Spawn MCP server
    let mut child = Command::new(&binary)
        // The server confines every path to this root.
        .env("FFM_MCP_ROOT", &tmp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start ffm-mcp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send initialize
    let init_req = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bench","version":"1.0"}}}"#;
    send(&mut stdin, init_req);
    let _ = recv(&mut reader); // initialize response

    // Send initialized notification
    let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send(&mut stdin, init_notif);

    // Small delay for server to be ready
    std::thread::sleep(Duration::from_millis(50));

    let mut results: Vec<(&str, Duration, bool)> = Vec::new();
    let mut req_id = 1u64;

    // Macro to benchmark a tool call
    macro_rules! bench_tool {
        ($name:expr, $args:expr) => {{
            let args_json = $args;
            let req = format!(
                r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{{"name":"{}","arguments":{}}}}}"#,
                req_id, $name, args_json
            );
            req_id += 1;
            let start = Instant::now();
            send(&mut stdin, &req);
            let resp = recv(&mut reader);
            let elapsed = start.elapsed();
            let ok = !resp.contains("\"isError\":true") && !resp.contains("\"error\":");
            results.push(($name, elapsed, ok));
        }};
    }

    // ── 1. list_directory ────────────────────────────────────────
    bench_tool!("list_directory", format!(r#"{{"path":"{}"}}"#, tmp_str));

    // ── 2. sort_directory ────────────────────────────────────────
    bench_tool!(
        "sort_directory",
        format!(r#"{{"path":"{}","sort":"size"}}"#, test_dir_str)
    );

    // ── 3. get_file_info ─────────────────────────────────────────
    bench_tool!(
        "get_file_info",
        format!(r#"{{"path":"{}"}}"#, test_file_str)
    );

    // ── 4. view_file ─────────────────────────────────────────────
    bench_tool!("view_file", format!(r#"{{"path":"{}"}}"#, test_file_str));

    // ── 5. create_file ───────────────────────────────────────────
    let new_file = tmp.join("created.txt");
    bench_tool!(
        "create_file",
        format!(
            r#"{{"path":"{}","content":"bench content"}}"#,
            new_file.display()
        )
    );

    // ── 6. write_file ────────────────────────────────────────────
    bench_tool!(
        "write_file",
        format!(
            r#"{{"path":"{}","content":"overwritten"}}"#,
            new_file.display()
        )
    );

    // ── 7. append_file ───────────────────────────────────────────
    bench_tool!(
        "append_file",
        format!(
            r#"{{"path":"{}","content":"\nappended"}}"#,
            new_file.display()
        )
    );

    // ── 8. create_directory ──────────────────────────────────────
    let new_dir = tmp.join("bench_newdir");
    bench_tool!(
        "create_directory",
        format!(r#"{{"path":"{}"}}"#, new_dir.display())
    );

    // ── 9. copy ──────────────────────────────────────────────────
    bench_tool!(
        "copy",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            test_file_str,
            copy_dest.join("copied.txt").display()
        )
    );

    // ── 10. move_rename ──────────────────────────────────────────
    bench_tool!(
        "move_rename",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            move_src.display(),
            tmp.join("moved.txt").display()
        )
    );

    // ── 11. delete ───────────────────────────────────────────────
    bench_tool!(
        "delete",
        format!(r#"{{"path":"{}"}}"#, delete_file.display())
    );

    // ── 12. search ───────────────────────────────────────────────
    bench_tool!(
        "search",
        format!(r#"{{"path":"{}","pattern":"txt"}}"#, tmp_str)
    );

    // ── 13. grep ─────────────────────────────────────────────────
    bench_tool!(
        "grep",
        format!(r#"{{"path":"{}","pattern":"content"}}"#, test_dir_str)
    );

    // ── 14. tree ─────────────────────────────────────────────────
    bench_tool!("tree", format!(r#"{{"path":"{}","depth":2}}"#, tmp_str));

    // ── 15. glob ─────────────────────────────────────────────────
    bench_tool!(
        "glob",
        format!(r#"{{"path":"{}","pattern":"*.txt"}}"#, tmp_str)
    );

    // ── 16. find ─────────────────────────────────────────────────
    bench_tool!(
        "find",
        format!(
            r#"{{"path":"{}","name":"*.txt","file_type":"file"}}"#,
            tmp_str
        )
    );

    // ── 17. replace ──────────────────────────────────────────────
    bench_tool!(
        "replace",
        format!(
            r#"{{"path":"{}","find":"old_text","replace":"new_text"}}"#,
            replace_dir.display()
        )
    );

    // ── 18. delete_by_pattern ────────────────────────────────────
    bench_tool!(
        "delete_by_pattern",
        format!(r#"{{"path":"{}","pattern":"*.tmp"}}"#, pat_dir.display())
    );

    // ── 19. list_drives ──────────────────────────────────────────
    bench_tool!("list_drives", "{}".to_string());

    // ── 20. disk_usage ───────────────────────────────────────────
    bench_tool!("disk_usage", format!(r#"{{"path":"{}"}}"#, test_dir_str));

    // ── 21. unique_name ──────────────────────────────────────────
    bench_tool!(
        "unique_name",
        format!(r#"{{"directory":"{}","filename":"test.txt"}}"#, tmp_str)
    );

    // ── 22. file_checksum ────────────────────────────────────────
    bench_tool!(
        "file_checksum",
        format!(r#"{{"path":"{}"}}"#, test_file_str)
    );

    // ── 23. format_size_tool ───────────────────────────────────────
    bench_tool!("format_size_tool", r#"{"bytes":1048576}"#.to_string());

    // ── 24. batch_copy ───────────────────────────────────────────
    bench_tool!(
        "batch_copy",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            batch1.display(),
            batch2.display(),
            batch_dest.display()
        )
    );

    // ── 25. batch_move ───────────────────────────────────────────
    bench_tool!(
        "batch_move",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            bmove1.display(),
            bmove2.display(),
            bmove_dest.display()
        )
    );

    // ── 26. batch_delete ─────────────────────────────────────────
    bench_tool!(
        "batch_delete",
        format!(
            r#"{{"paths":["{}","{}"]}}"#,
            bdel1.display(),
            bdel2.display()
        )
    );

    // ── 27. create_symlink ───────────────────────────────────────
    bench_tool!(
        "create_symlink",
        format!(
            r#"{{"link":"{}","target":"{}"}}"#,
            symlink_link.display(),
            symlink_target.display()
        )
    );

    // ── 28. read_symlink ─────────────────────────────────────────
    bench_tool!(
        "read_symlink",
        format!(r#"{{"path":"{}"}}"#, symlink_link.display())
    );

    // ── 29. permissions ──────────────────────────────────────────
    bench_tool!("permissions", format!(r#"{{"path":"{}"}}"#, test_file_str));

    // ── 30. set_bookmark ─────────────────────────────────────────
    bench_tool!(
        "set_bookmark",
        format!(r#"{{"name":"bench_test","path":"{}"}}"#, tmp_str)
    );

    // ── 31. get_bookmark ─────────────────────────────────────────
    bench_tool!("get_bookmark", r#"{"name":"bench_test"}"#.to_string());

    // ── 32. list_bookmarks ───────────────────────────────────────
    bench_tool!("list_bookmarks", "{}".to_string());

    // ── 33. delete_bookmark ──────────────────────────────────────
    bench_tool!("delete_bookmark", r#"{"name":"bench_test"}"#.to_string());

    // ── 34. system_info ──────────────────────────────────────────
    bench_tool!("system_info", "{}".to_string());

    // ── 35. s3_list_profiles ─────────────────────────────────────
    bench_tool!("s3_list_profiles", "{}".to_string());

    // S3 tools that require real AWS credentials — skip with note
    let s3_skipped = [
        "s3_list_buckets",
        "s3_list_objects",
        "s3_download",
        "s3_upload",
        "s3_delete",
        "s3_copy",
    ];

    // Print results
    println!("┌─────────────────────────┬───────────┬────────┐");
    println!("│ Tool                    │ Time      │ Status │");
    println!("├─────────────────────────┼───────────┼────────┤");

    let mut total = Duration::ZERO;
    let mut pass_count = 0u32;
    let mut fail_count = 0u32;

    for (name, dur, ok) in &results {
        let status = if *ok { "  OK  " } else { " FAIL " };
        let time_str = format_duration(*dur);
        println!("│ {:<23} │ {:>9} │{}│", name, time_str, status);
        total += *dur;
        if *ok {
            pass_count += 1;
        } else {
            fail_count += 1;
        }
    }

    for name in &s3_skipped {
        println!("│ {:<23} │   skipped │  S3   │", name);
    }

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
    println!(
        "Skipped: {} S3 tools (need AWS credentials)",
        s3_skipped.len()
    );

    // Compute stats
    if !results.is_empty() {
        let times: Vec<f64> = results
            .iter()
            .map(|(_, d, _)| d.as_secs_f64() * 1000.0)
            .collect();
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(0.0f64, f64::max);
        let min_name = results.iter().min_by_key(|(_, d, _)| *d).unwrap().0;
        let max_name = results.iter().max_by_key(|(_, d, _)| *d).unwrap().0;
        println!();
        println!("Stats:");
        println!("  Avg: {:.2} ms", avg);
        println!("  Min: {:.2} ms ({})", min, min_name);
        println!("  Max: {:.2} ms ({})", max, max_name);
        println!("  Total: {:.2} ms", total.as_secs_f64() * 1000.0);
    }

    if fail_count > 0 {
        println!();
        println!("FAILURES:");
        for (name, _, ok) in &results {
            if !ok {
                println!("  - {}", name);
            }
        }
    }

    // Cleanup
    let _ = stdin.write_all(b"");
    let _ = child.kill();
    let _ = std::fs::remove_dir_all(&tmp);
}

fn send(stdin: &mut impl Write, msg: &str) {
    // rmcp uses line-delimited JSON (no Content-Length header)
    stdin.write_all(msg.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    stdin.flush().unwrap();
}

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
        format!("{:.2} ms", ms)
    } else {
        format!("{:.2} s", ms / 1000.0)
    }
}
