//! Purpose: Check rendered provider rows, including explicit unavailable totals for failures.

use super::BehaviorWorld;
use cucumber::then;

#[then(expr = "the token usage row for {string} contains {string}")]
fn totals(world: &mut BehaviorWorld, provider: String, expected: String) {
    let screen = world.case().screen.rows();
    let row = provider_row(&screen, &provider);
    let cells: Vec<_> = row.split_whitespace().collect();
    let expected: Vec<_> = expected.split_whitespace().collect();
    assert_eq!(&cells[1..], expected.as_slice(), "{row}");
}

#[then(expr = "the token usage row for {string} reports {string} without numeric totals")]
fn failure(world: &mut BehaviorWorld, provider: String, expected: String) {
    let screen = world.case().screen.rows();
    let row = provider_row(&screen, &format!("{provider}:"));
    assert!(row.contains(&expected), "{row}");
    assert!(
        row.split_whitespace()
            .all(|word| word.parse::<u64>().is_err()),
        "{row}"
    );
}

fn provider_row<'a>(screen: &'a [String], provider: &str) -> &'a str {
    screen
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .find(|line| line.starts_with(provider))
        .unwrap_or_else(|| panic!("provider {provider} is missing from: {screen:?}"))
}
