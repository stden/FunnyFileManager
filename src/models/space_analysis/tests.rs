//! Unit tests: the Space analysis dialog's state — ranking, the cut to the
//! shown rows, the totals, the shares and the cursor.

use super::*;
use crate::space_verdict::{Keep, Regenerable};

fn kept(name: &str, bytes: u64) -> SpaceEntry {
    SpaceEntry {
        name: name.to_owned(),
        bytes,
        verdict: Verdict::Keep(Keep::Tracked),
    }
}

fn judged(name: &str, bytes: u64, verdict: Verdict) -> SpaceEntry {
    SpaceEntry {
        name: name.to_owned(),
        bytes,
        verdict,
    }
}

fn dialog(entries: Vec<SpaceEntry>) -> SpaceAnalysisDialog {
    SpaceAnalysisDialog::new(PathBuf::from("/work"), entries, String::new())
}

#[test]
fn rows_are_largest_first_and_ties_go_by_name() {
    let dialog = dialog(vec![kept("b", 5), kept("a", 5), kept("big", 9)]);
    let names: Vec<&str> = dialog.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["big", "a", "b"]);
}

#[test]
fn only_the_largest_shown_entries_are_listed() {
    let entries = (0..30)
        .map(|index| kept(&format!("e{index:02}"), 1000 + index))
        .collect();
    let dialog = dialog(entries);
    assert_eq!(dialog.rows.len(), SHOWN);
    assert_eq!(dialog.rows.first().map(|row| row.bytes), Some(1029));
    assert_eq!(dialog.rows.last().map(|row| row.bytes), Some(1005));
}

#[test]
fn the_reclaimable_total_counts_every_safe_entry_not_only_the_listed_ones() {
    let mut entries: Vec<SpaceEntry> = (0..30)
        .map(|index| kept(&format!("k{index:02}"), 1_000_000 + index))
        .collect();
    entries.push(judged("target", 7, Verdict::Safe(Regenerable::CargoTarget)));
    entries.push(judged(
        "__pycache__",
        3,
        Verdict::Safe(Regenerable::PythonBytecode),
    ));
    let dialog = dialog(entries);
    assert_eq!(dialog.reclaimable.bytes(), 10);
    assert!(
        dialog.rows.iter().all(|row| row.name != "target"),
        "counted, but not listed"
    );
}

#[test]
fn the_folder_total_is_the_sum_of_every_entry() {
    let dialog = dialog(vec![kept("a", 30), kept("b", 70)]);
    assert_eq!(dialog.total.bytes(), 100);
    assert_eq!(dialog.reclaimable.bytes(), 0, "nothing is safe here");
}

#[test]
fn a_share_is_a_whole_percentage_of_the_folder_total() {
    let dialog = dialog(vec![kept("a", 25), kept("b", 75)]);
    assert_eq!(dialog.rows[0].share, 75);
    assert_eq!(dialog.rows[1].share, 25);
    assert_eq!(share_percent(0, 0), 0, "an empty folder has no shares");
    assert_eq!(share_percent(1, 3), 33, "rounded down");
}

#[test]
fn the_cursor_moves_within_the_rows_and_stops_at_the_ends() {
    let mut dialog = dialog(vec![kept("a", 3), kept("b", 2), kept("c", 1)]);
    assert_eq!(dialog.selected_name(), Some("a"));
    dialog.move_to(2);
    assert_eq!(dialog.selected_name(), Some("c"));
    dialog.move_to(99);
    assert_eq!(dialog.cursor, 2, "clamped to the last row");
}

#[test]
fn an_empty_folder_has_no_row_to_select() {
    let mut dialog = dialog(Vec::new());
    dialog.move_to(3);
    assert_eq!(dialog.selected_name(), None);
    assert_eq!(dialog.cursor, 0);
}

#[test]
fn the_note_is_kept_as_given() {
    let dialog = SpaceAnalysisDialog::new(
        PathBuf::from("/work"),
        Vec::new(),
        "The entry budget ran out".to_owned(),
    );
    assert_eq!(dialog.note, "The entry budget ran out");
    assert_eq!(dialog.folder, PathBuf::from("/work"));
}
