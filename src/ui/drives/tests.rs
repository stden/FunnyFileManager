//! Unit tests: the Alt+F1 / Alt+F2 "Change drive" dialog.
//!
//! The rows are asserted as text rather than by eye: what matters is that the
//! box closes, that the columns line up under their separators, and that the
//! numbers are right-aligned the way Far writes them.

use super::table::{body, cell};
use super::{Widths, FOOTER};
use crate::diskfree::{Capacity, DriveKind};
use crate::models::{DriveEntry, DriveRow, RowKind};
use crate::ui::panel::{pad_cells, right_cells};
use crate::ui::Cells;

pub(super) fn disk(label: &str, device: DriveKind, total: u64, free: u64) -> DriveRow {
    DriveRow {
        key: label.chars().next(),
        kind: RowKind::Disk {
            label: label.to_owned(),
            device,
            space: Some(Capacity {
                total,
                available: free,
            }),
        },
        entry: DriveEntry::Local(format!("{label}\\").into()),
    }
}

pub(super) fn empty_bay(label: &str) -> DriveRow {
    DriveRow {
        key: label.chars().next(),
        kind: RowKind::Disk {
            label: label.to_owned(),
            device: DriveKind::CdRom,
            space: None,
        },
        entry: DriveEntry::Local(format!("{label}\\").into()),
    }
}

pub(super) fn plugin(key: char, name: &str) -> DriveRow {
    DriveRow {
        key: Some(key),
        kind: RowKind::Plugin {
            name: name.to_owned(),
        },
        entry: DriveEntry::S3Profile(name.to_owned()),
    }
}

/// Render the body to plain text, one string per line.
pub(super) fn text(rows: &[DriveRow], selected: usize) -> Vec<String> {
    let widths = Widths::of(rows);
    body(rows, &widths, selected)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

/// The shape from Far: a boxed table, one row per drive, sizes on the right.
#[test]
fn a_single_drive_is_boxed_with_its_size_and_free_space() {
    let lines = text(
        &[disk(
            "C:",
            DriveKind::Fixed,
            500_000_000_000,
            52_000_000_000,
        )],
        0,
    );

    assert_eq!(
        lines,
        vec![
            "┌──────────┬───────┬────────┐".to_owned(),
            "│ C: fixed │ 466 G │ 48.4 G │".to_owned(),
            "└──────────┴───────┴────────┘".to_owned(),
        ]
    );
}

/// Every line of the box is the same width, or the frame does not close.
#[test]
fn every_line_is_the_same_width() {
    let rows = [
        disk("C:", DriveKind::Fixed, 500_000_000_000, 52_000_000_000),
        disk("D:", DriveKind::Removable, 8_000_000_000, 100),
        plugin('0', "S3: a-rather-long-profile-name"),
    ];
    let lines = text(&rows, 0);
    let width = lines[0].chars().count();
    for line in &lines {
        assert_eq!(line.chars().count(), width, "{line:?}");
    }
}

/// The rule between the sections closes the separators above it and opens
/// none below: `┴`, not `┬`.
#[test]
fn the_rule_closes_the_columns_above_it() {
    let rows = [
        disk("C:", DriveKind::Fixed, 100, 50),
        plugin('0', "S3: work"),
    ];
    let lines = text(&rows, 0);

    let rule = lines
        .iter()
        .find(|line| line.starts_with('├'))
        .expect("a rule");
    assert!(rule.contains('┴'), "{rule:?}");
    assert!(!rule.contains('┬'), "{rule:?}");
    assert!(lines.last().expect("closed").starts_with('└'));
    assert!(
        !lines.last().expect("closed").contains('┴'),
        "nothing below the plugins to close"
    );
}

/// With no plugin rows the bottom edge is the one that closes the columns.
#[test]
fn without_plugins_the_bottom_edge_closes_the_columns() {
    let lines = text(&[disk("C:", DriveKind::Fixed, 100, 50)], 0);
    assert!(lines.last().expect("closed").contains('┴'));
    assert!(!lines.iter().any(|line| line.starts_with('├')));
}

/// The alignment comes from the panel's own cell helpers; text already at its
/// column width passes through, and wider text cannot arrive because every
/// width is the maximum `Widths::of` took over these same rows.
#[test]
fn names_pad_right_and_sizes_pad_left() {
    assert_eq!(pad_cells("ab", Cells::new(5)), "ab   ");
    assert_eq!(right_cells("ab", Cells::new(5)), "   ab");
    assert_eq!(pad_cells("abcdef", Cells::new(6)), "abcdef");
    assert_eq!(right_cells("abcdef", Cells::new(6)), "abcdef");
    assert_eq!(cell("x"), " x ");
}

/// The footer sets a floor on the width, so the legend in the bottom border is
/// never clipped by a narrow set of drives. The fixture's table really is
/// narrower than the legend — remove the floor and the box shrinks to it.
#[test]
fn the_box_is_never_narrower_than_its_own_legend() {
    // One drive, an unnamed kind, no size: the narrowest table there is.
    let rows = [DriveRow {
        key: Some('/'),
        kind: RowKind::Disk {
            label: "/".to_owned(),
            device: DriveKind::Unknown,
            space: None,
        },
        entry: DriveEntry::Local("/".into()),
    }];
    let widths = Widths::of(&rows);
    let legend = FOOTER.chars().count();
    assert!(
        widths.table() < legend,
        "the fixture must be narrower than the legend, got {}",
        widths.table()
    );
    assert_eq!(widths.interior(), legend, "the legend is the floor");
    for line in text(&rows, 0) {
        // The interior plus the box's own `│` either side.
        assert_eq!(line.chars().count(), legend + 2, "{line:?}");
    }
}

/// An empty machine still produces a closed box rather than a panic.
#[test]
fn no_drives_at_all_still_closes_the_box() {
    let lines = text(&[], 0);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
    assert!(lines[1].starts_with('└') && lines[1].ends_with('┘'));
    assert_eq!(lines[0].chars().count(), lines[1].chars().count());
}
