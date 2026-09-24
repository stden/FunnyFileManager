//! Purpose: `Then` steps that check the lists a tool call returned:
//! counts, entry names and order, profiles, plain lists and grep matches.

use crate::world::McpWorld;
use cucumber::then;
use serde_json::Value;

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
