#!/usr/bin/env rust-script
//! Benchmark all FFM MCP server tools via JSON-RPC over stdio.
//!
//! Usage:
//!   `cargo build --release -j1 && cargo run --example bench_mcp`
//! Or standalone:
//!   `rustc --edition 2021 bench_mcp.rs -o bench_mcp && ./bench_mcp`
//!
//! `bench_mcp/server.rs` starts the server and speaks its line protocol,
//! `fixtures.rs` lays out the files the calls work on, `suite.rs` times
//! every call of the `*_tools.rs` groups, and `report.rs` prints the table.

use std::io::BufReader;
use std::process::ExitCode;
use std::time::Duration;

use fixtures::create_fixtures;
use report::print_results;
use server::{locate_server, recv, send, start_server};
use suite::benchmark_tools;

#[path = "bench_mcp/batch_tools.rs"]
mod batch_tools;
#[path = "bench_mcp/file_tools.rs"]
mod file_tools;
#[path = "bench_mcp/fixtures.rs"]
mod fixtures;
#[path = "bench_mcp/host_tools.rs"]
mod host_tools;
#[path = "bench_mcp/report.rs"]
mod report;
#[path = "bench_mcp/search_tools.rs"]
mod search_tools;
#[path = "bench_mcp/server.rs"]
mod server;
#[path = "bench_mcp/suite.rs"]
mod suite;

#[cfg(test)]
#[path = "bench_mcp/tests.rs"]
mod tests;

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
