//! Purpose: `Then` steps that check the last tool call's outcome: result
//! text, scalar fields and flags, errors, and how the server exited.

use crate::world::McpWorld;
use cucumber::then;
use std::path::Path;

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
