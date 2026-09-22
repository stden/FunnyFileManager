//! Unit tests: the Alt+F1 / Alt+F2 "Change drive" dialog.
//!
//! The rows are asserted as text rather than by eye: what matters is that the
//! box closes, that the columns line up under their separators, and that the
//! numbers are right-aligned the way Far writes them.

use super::{body, cell, centered, columns, Widths, FOOTER};
use crate::diskfree::{Capacity, DriveKind};
use crate::models::{DriveEntry, DriveRow, RowKind};
use crate::ui::panel::{pad_cells, right_cells};
use crate::ui::Cells;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

fn disk(label: &str, device: DriveKind, total: u64, free: u64) -> DriveRow {
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

fn empty_bay(label: &str) -> DriveRow {
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

fn plugin(key: char, name: &str) -> DriveRow {
    DriveRow {
        key: Some(key),
        kind: RowKind::Plugin {
            name: name.to_owned(),
        },
        entry: DriveEntry::S3Profile(name.to_owned()),
    }
}

/// Render the body to plain text, one string per line.
fn text(rows: &[DriveRow], selected: usize) -> Vec<String> {
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

/// A plugin row shows its key, and it is wide enough that the box still closes.
#[test]
fn a_plugin_row_shows_its_key_and_name() {
    let rows = [
        disk("C:", DriveKind::Fixed, 100, 50),
        plugin('2', "S3: work"),
    ];
    let lines = text(&rows, 1);
    let row = lines
        .iter()
        .find(|line| line.contains("S3: work"))
        .expect("the plugin row");
    assert!(row.contains(" 2 "), "{row:?}");
}

/// A drive that will not report its size leaves the columns blank rather than
/// claiming zero bytes — an empty bay is not a full disk.
#[test]
fn a_drive_with_no_answer_shows_blank_columns_not_zero() {
    let lines = text(&[empty_bay("E:")], 0);
    let row = &lines[1];
    assert!(row.contains("E: CD-ROM"), "{row:?}");
    assert!(!row.contains('0'), "{row:?}");
}

/// A kind the platform would not name contributes nothing to the label.
#[test]
fn an_unknown_kind_leaves_the_label_bare() {
    let cols = columns(&RowKind::Disk {
        label: "X:".to_owned(),
        device: DriveKind::Unknown,
        space: None,
    })
    .expect("a disk row has columns");
    assert_eq!(cols.label, "X:");
    assert!(cols.total.is_empty());
}

#[test]
fn a_plugin_row_has_no_columns() {
    assert!(columns(&RowKind::Plugin {
        name: "S3: work".to_owned()
    })
    .is_none());
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

/// The dialog takes its natural size, and never more than the screen.
#[test]
fn the_dialog_is_centred_and_clamped_to_the_screen() {
    let screen = Rect::new(0, 0, 80, 24);
    let area = centered(screen, 30, 6);
    assert_eq!(area.width, 30);
    assert_eq!(area.height, 6);
    assert_eq!(area.x, 25);
    assert_eq!(area.y, 9);

    let squeezed = centered(Rect::new(0, 0, 10, 3), 40, 20);
    assert_eq!((squeezed.width, squeezed.height), (10, 3));
    assert_eq!((squeezed.x, squeezed.y), (0, 0));
}

/// The whole dialog draws without panicking, and the title reaches the screen.
#[test]
fn the_dialog_draws_its_title_and_legend() {
    let rows = [disk("C:", DriveKind::Fixed, 100, 50)];
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test terminal");
    terminal
        .draw(|f| super::render(f, crate::models::PanelSide::Left, &rows, 0))
        .expect("draw");
    let screen: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(screen.contains("Change drive"), "the title is drawn");
    assert!(screen.contains("Enter"), "the legend is drawn");
}

/// The selection bar runs edge to edge even where the box is wider than the
/// table: the filler after the free column takes the row's background, not
/// the frame's. With the frame's ground there it stopped mid-row with a gap
/// whenever a plugin row or the footer widened the box.
#[test]
fn a_chosen_disk_row_is_highlighted_to_the_edge() {
    // The long plugin name makes interior() wider than table(), so the disk
    // rows have a filler to paint.
    let rows = [
        disk("C:", DriveKind::Fixed, 500_000_000_000, 52_000_000_000),
        plugin('0', "S3: a-rather-long-profile-name"),
    ];
    let widths = Widths::of(&rows);
    assert!(
        widths.interior() > widths.table(),
        "the fixture must leave a filler to paint"
    );

    let filler_backgrounds = |selected: usize| {
        body(&rows, &widths, selected)
            .into_iter()
            .map(|line| {
                line.spans
                    .iter()
                    .find(|span| {
                        !span.content.is_empty()
                            && span.content.chars().all(|ch| ch == ' ')
                            && span.content.len() > 2
                    })
                    .and_then(|span| span.style.bg)
            })
            .collect::<Vec<_>>()
    };
    // Line 1 is the disk row; chosen, its filler carries the selection bar.
    assert_eq!(
        filler_backgrounds(0)[1],
        Some(crate::ui::palette::DRIVE_SELECTED_BG)
    );
    // Not chosen, the filler falls back to the menu ground.
    assert_eq!(filler_backgrounds(2)[1], Some(crate::ui::palette::MENU_BG));
}

/// Past the tenth profile a row has no shortcut; it still renders, with a
/// blank hotkey cell so its name lines up with the keyed rows above.
#[test]
fn a_keyless_plugin_row_renders_a_blank_hotkey_cell() {
    let mut rows = vec![disk("C:", DriveKind::Fixed, 100, 50)];
    rows.extend((0..11).map(|n| plugin(char::from_digit(n % 10, 10).expect("digit"), "S3: p")));
    rows[11].key = None;
    let lines = text(&rows, 0);

    let width = lines[0].chars().count();
    for line in &lines {
        assert_eq!(line.chars().count(), width, "{line:?}");
    }
    let keyless = &lines[lines.len() - 2];
    assert!(keyless.starts_with("│   S3: p"), "{keyless:?}");
    let keyed = &lines[lines.len() - 3];
    assert!(keyed.starts_with("│ 9 S3: p"), "{keyed:?}");
}

/// The cursor row is the one that changes style; nothing else does.
#[test]
fn only_the_selected_row_is_highlighted() {
    let rows = [
        disk("C:", DriveKind::Fixed, 100, 50),
        disk("D:", DriveKind::Fixed, 100, 50),
    ];
    let widths = Widths::of(&rows);
    let highlighted = |selected: usize| {
        body(&rows, &widths, selected)
            .into_iter()
            .map(|line| {
                line.spans
                    .iter()
                    .any(|span| span.style.fg == Some(crate::ui::palette::HOTKEY))
            })
            .collect::<Vec<_>>()
    };
    // Line 0 is the top edge, so the drives are lines 1 and 2.
    assert_eq!(highlighted(0), vec![false, true, false, false]);
    assert_eq!(highlighted(1), vec![false, false, true, false]);
}
