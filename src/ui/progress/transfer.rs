//! Purpose: The rows of the transfer dialog of a copy, a move or a delete: the
//! file being sent, where it goes, its bar, and the totals of the whole
//! operation with its time and rate.

use std::path::Path;

use ratatui::text::Line;

use super::text::{bar, centred_rule, clock, left_right};
use crate::models::FileSize;
use crate::progress::{Operation, Phase, Snapshot};
use crate::ui::cells::{middle_cells, tail_cells};
use crate::ui::panel::group_digits;
use crate::ui::{printable_name, Cells};

/// The dialog's rows inside its frame, `width` cells wide: the heading, the
/// file's path, its destination, its bar, the totals and the time.
pub(super) fn rows(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    let (to, destination) = match &snapshot.transfer {
        Some(transfer) => ("to", destination_text(&transfer.destination, width)),
        None => ("", String::new()),
    };
    vec![
        Line::from(super::heading(snapshot.phase, snapshot.operation)),
        Line::from(current_text(&snapshot.current, width)),
        Line::from(to),
        Line::from(destination),
        Line::from(meter(snapshot, width)),
        Line::from(total_meter(snapshot, width)),
        Line::from(centred_rule("Total", width)),
        Line::from(left_right(
            "Files:",
            &count_text(snapshot.done.files, snapshot.total.files, snapshot.phase),
            width,
        )),
        Line::from(left_right("Bytes:", &bytes_text(snapshot), width)),
        Line::from(left_right(
            &time_text(snapshot),
            &rate_text(snapshot),
            width,
        )),
    ]
}

/// Preserve progress and time on a terminal too short for the full dialog.
pub(super) fn compact_rows(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    let mut rows = vec![Line::from(current_text(&snapshot.current, width))];
    if let Some(transfer) = &snapshot.transfer {
        rows.push(Line::from(format!(
            "to {}",
            destination_text(&transfer.destination, width.saturating_sub(3))
        )));
    } else {
        rows.push(Line::from(super::heading(
            snapshot.phase,
            snapshot.operation,
        )));
    }
    rows.push(Line::from(meter(snapshot, width)));
    rows.push(Line::from(total_meter(snapshot, width)));
    let files = count_text(snapshot.done.files, snapshot.total.files, snapshot.phase);
    let bytes = bytes_text(snapshot);
    let counts = if bytes.is_empty() {
        format!("Files: {files}")
    } else {
        format!("Files: {files}  Bytes: {bytes}")
    };
    rows.push(Line::from(counts));
    rows.push(Line::from(left_right(
        &time_text(snapshot),
        &rate_text(snapshot),
        width,
    )));
    rows
}

/// The file's path as the dialog has always shown it, cut from its start to fit
/// the row, so the file name at its end stays in view.
fn current_text(path: &Path, width: usize) -> String {
    let label = crate::path_display::label(path);
    tail_cells(&printable_name(&label), Cells::new(width))
}

/// The file's destination, as its label reads, cut to fit the row.
fn destination_text(destination: &Path, width: usize) -> String {
    let label = crate::path_display::label(destination);
    middle_cells(&printable_name(&label), Cells::new(width))
}

/// The bar of the file being sent, or of the whole operation when it sends no
/// file (a delete); nothing while the operation is still measuring.
fn meter(snapshot: &Snapshot, width: usize) -> String {
    match (snapshot.phase, &snapshot.transfer) {
        (Phase::Scanning, _) => String::new(),
        (Phase::Working, Some(transfer)) => bar(transfer.file.permille(), width),
        (Phase::Working, None) => bar(snapshot.permille(), width),
    }
}

fn total_meter(snapshot: &Snapshot, width: usize) -> String {
    if snapshot.phase == Phase::Working && snapshot.transfer.is_some() {
        format!(
            "Total {}",
            bar(snapshot.permille(), width.saturating_sub(6))
        )
    } else {
        String::new()
    }
}

fn count_text(done: u64, total: u64, phase: Phase) -> String {
    if phase == Phase::Scanning {
        group_digits(total)
    } else {
        format!("{} of {}", group_digits(done), group_digits(total))
    }
}

/// The bytes of the whole operation, grouped as Far prints them. A delete reads
/// no bytes, so it has none to total.
fn bytes_text(snapshot: &Snapshot) -> String {
    if snapshot.operation == Operation::Delete {
        return String::new();
    }
    count_text(snapshot.done.bytes, snapshot.total.bytes, snapshot.phase)
}

/// `Time: 00:00:40`: the time so far.
fn time_text(snapshot: &Snapshot) -> String {
    format!("Time: {}", clock(snapshot.elapsed.as_secs()))
}

/// `Speed: 93.1 KB/s`: the average rate so far, in `FileSize`'s units.
fn rate_text(snapshot: &Snapshot) -> String {
    let rate = FileSize::from_bytes(snapshot.bytes_per_second());
    format!("Speed: {}/s", rate.one_decimal())
}
