//! Purpose: The progress dialog of a long copy, move, delete or search, drawn on its
//! own into a buffer the size of the dialog: the operation holds `&mut App`
//! while it runs, so the dialog is painted over the last frame rather than
//! as part of a new one (see `progress`).

mod text;
mod transfer;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph, Widget},
};

use super::cells::{printable_name, tail_cells};
use super::{dialog_frame, palette, Cells};
use crate::copy::Purpose;
use crate::progress::{Operation, Phase, Snapshot};
use text::{bar, clock};

/// The widest the dialog grows, frame included; a narrower screen gets a
/// narrower dialog.
const MAX_WIDTH: u16 = 60;
/// The frame and the padding inside it, on both sides.
const CHROME: u16 = 4;
/// The top and the bottom border of the frame.
const BORDER_ROWS: u16 = 2;

/// The progress dialog for `snapshot`, centred on a screen of `screen`'s
/// size. Every cell of the returned buffer belongs to the dialog.
#[must_use]
pub fn progress_dialog(screen: Rect, snapshot: &Snapshot) -> Buffer {
    let width = dialog_width(screen);
    let text_width = usize::from(width.saturating_sub(CHROME));
    let rows = if screen.height < 12
        && matches!(
            snapshot.operation,
            Operation::Copy | Operation::Move | Operation::Delete
        ) {
        transfer::compact_rows(snapshot, text_width)
    } else {
        lines(snapshot, text_width)
    };
    let area = dialog_area(screen, width, rows.len());
    let mut buffer = Buffer::empty(area);
    let ground = Style::default()
        .fg(palette::DIALOG_FG)
        .bg(palette::DIALOG_BG);
    let frame_title = title(snapshot.operation);
    Clear.render(area, &mut buffer);
    Paragraph::new(rows)
        .block(dialog_frame(&frame_title, "", ground))
        .style(ground)
        .render(area, &mut buffer);
    buffer
}

/// As wide as the screen allows, up to [`MAX_WIDTH`].
fn dialog_width(screen: Rect) -> u16 {
    screen.width.saturating_sub(2).min(MAX_WIDTH)
}

/// Centred, `width` wide, and as tall as its `rows` of text with the borders
/// around them, or as tall as the screen when that is less.
fn dialog_area(screen: Rect, width: u16, rows: usize) -> Rect {
    let rows = u16::try_from(rows).unwrap_or(u16::MAX);
    let height = rows.saturating_add(BORDER_ROWS).min(screen.height);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

/// F5 and F6 are titled as their dialog is: "Copy" and "Rename/Move".
fn title(operation: Operation) -> String {
    let name = match operation {
        Operation::Copy => Purpose::Copy.title(),
        Operation::Move => Purpose::Move.title(),
        Operation::Delete => "Delete",
        Operation::Find => "Find",
        Operation::GitPull => "Git pull",
    };
    format!(" {name} ")
}

/// What the dialog says is happening.
fn heading(phase: Phase, operation: Operation) -> &'static str {
    match (phase, operation) {
        (_, Operation::Find) => "Searching in",
        (Phase::Scanning, Operation::GitPull) => "Finding repositories (Esc / Ctrl+C: cancel)",
        (Phase::Scanning, Operation::Copy | Operation::Move | Operation::Delete) => "Scanning",
        (Phase::Working, Operation::Copy) => "Copying the file",
        (Phase::Working, Operation::Move) => "Moving the file",
        (Phase::Working, Operation::Delete) => "Deleting the file",
        (Phase::Working, Operation::GitPull) => "Updating repositories (Esc / Ctrl+C: cancel)",
    }
}

fn lines(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    match snapshot.operation {
        Operation::Copy | Operation::Move | Operation::Delete => transfer::rows(snapshot, width),
        Operation::Find => search_rows(snapshot, width),
        Operation::GitPull => pull_rows(snapshot, width),
    }
}

/// A search has no total to fill a bar with: it says where it is, and how many
/// matches it found and entries it looked at.
fn search_rows(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    let counts = format!(
        "Found: {}   Checked: {}",
        snapshot.done.files, snapshot.total.files
    );
    plain_rows(snapshot, width, String::new(), counts)
}

/// A Git pull counts its repositories, and fills a bar once it updates them.
fn pull_rows(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    let (meter, counts) = match snapshot.phase {
        Phase::Scanning => (
            String::new(),
            format!("Repositories found: {}", snapshot.total.files),
        ),
        Phase::Working => (
            bar(snapshot.permille(), width),
            format!(
                "Repositories: {} of {}",
                snapshot.done.files, snapshot.total.files
            ),
        ),
    };
    plain_rows(snapshot, width, meter, counts)
}

/// The rows of a search or a Git pull: what it does, where it is, its bar and
/// counts, and the time.
fn plain_rows(
    snapshot: &Snapshot,
    width: usize,
    meter: String,
    counts: String,
) -> Vec<Line<'static>> {
    let current = printable_name(&crate::path_display::label(&snapshot.current)).into_owned();
    vec![
        Line::from(heading(snapshot.phase, snapshot.operation)),
        Line::from(tail_cells(&current, Cells::new(width))),
        Line::from(""),
        Line::from(meter),
        Line::from(counts),
        Line::from(format!("Time: {}", clock(snapshot.elapsed.as_secs()))),
    ]
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_git_pull;

#[cfg(test)]
mod tests_scan_heading;

#[cfg(test)]
mod tests_text;

#[cfg(test)]
mod tests_transfer;
