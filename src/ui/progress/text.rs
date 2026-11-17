//! Purpose: The text the progress dialog is made of: a bar with its percentage,
//! a clock, a row with its two ends apart, and a rule with a caption centred in it.

use crate::ui::Cells;

/// `█████░░░░░  42%`, as wide as `width`.
pub(super) fn bar(permille: u16, width: usize) -> String {
    let percent = format!("{:>4}%", permille / 10);
    let cells = width.saturating_sub(percent.len() + 1);
    let filled = cells * usize::from(permille) / 1000;
    format!(
        "{}{} {percent}",
        "█".repeat(filled),
        "░".repeat(cells - filled)
    )
}

/// `hh:mm:ss`.
pub(super) fn clock(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// `left` at the start of a row `width` cells wide and `right` at its end, with
/// a gap between them when the row is too narrow for both at its ends.
pub(super) fn left_right(left: &str, right: &str, width: usize) -> String {
    let used = Cells::of(left).get() + Cells::of(right).get();
    let gap = width.saturating_sub(used).max(1);
    format!("{left}{}{right}", " ".repeat(gap))
}

/// A rule `width` cells wide with `caption` centred in it: `──── Total ────`.
pub(super) fn centred_rule(caption: &str, width: usize) -> String {
    let caption = format!(" {caption} ");
    let fill = width.saturating_sub(Cells::of(&caption).get());
    let left = fill / 2;
    format!(
        "{}{caption}{}",
        "─".repeat(left),
        "─".repeat(fill.saturating_sub(left))
    )
}
