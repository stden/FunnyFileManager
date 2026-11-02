//! Purpose: Times one JSON-RPC call per tool, group by group, and escapes
//! the paths those calls carry.

use super::batch_tools::batch_operations;
use super::file_tools::file_operations;
use super::host_tools::{metadata_operations, system_operations};
use super::search_tools::search_operations;
use super::server::{recv, send};
use std::fmt::Write as _;
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub fn benchmark_tools(
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
pub fn json_path(path: &Path) -> String {
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
