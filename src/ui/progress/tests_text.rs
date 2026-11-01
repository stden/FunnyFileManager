//! Purpose: The progress dialog's text helpers: the bar, a row with its two ends
//! apart, the centred rule, and the thousands of a byte count.

use super::text::{bar, centred_rule, left_right};
use crate::ui::panel::group_digits;

#[test]
fn the_bar_fills_its_width() {
    assert_eq!(bar(0, 10), "░░░░    0%");
    assert_eq!(bar(500, 10), "██░░   50%");
    assert_eq!(bar(1000, 10), "████  100%");
}

#[test]
fn the_two_ends_of_a_row_sit_at_its_edges() {
    assert_eq!(
        left_right("Files:", "467", 20),
        format!("Files:{}467", " ".repeat(11))
    );
    assert_eq!(
        left_right("Time:", "Remaining: x", 5),
        "Time: Remaining: x",
        "a row too narrow for both keeps a gap between them"
    );
}

#[test]
fn a_rule_centres_its_caption() {
    assert_eq!(
        centred_rule("Total", 20),
        format!("{}{}{}", "─".repeat(6), " Total ", "─".repeat(7))
    );
}

#[test]
fn a_byte_count_is_grouped_by_thousands_as_far_prints_it() {
    assert_eq!(group_digits(0), "0");
    assert_eq!(group_digits(999), "999");
    assert_eq!(group_digits(1000), "1,000");
    assert_eq!(group_digits(47_125_964), "47,125,964");
}
