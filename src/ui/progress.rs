//! Purpose: The progress dialog of a long copy, move, delete or search, drawn on its
//! own into a buffer the size of the dialog: the operation holds `&mut App`
//! while it runs, so the dialog is painted over the last frame rather than
//! as part of a new one (see `progress`).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{cells::printable_name, dialog_frame, palette};
use crate::models::FileSize;
use crate::progress::{Operation, Phase, Snapshot};

/// The widest the dialog grows, frame included; a narrower screen gets a
/// narrower dialog.
const MAX_WIDTH: u16 = 76;
/// The frame and the padding inside it, on both sides.
const CHROME: u16 = 4;
/// Six rows of text inside the frame.
const HEIGHT: u16 = 8;

/// The progress dialog for `snapshot`, centred on a screen of `screen`'s
/// size. Every cell of the returned buffer belongs to the dialog.
#[must_use]
pub fn progress_dialog(screen: Rect, snapshot: &Snapshot) -> Buffer {
    let area = dialog_area(screen);
    let mut buffer = Buffer::empty(area);
    let text_width = usize::from(area.width.saturating_sub(CHROME));
    let ground = Style::default()
        .fg(palette::DIALOG_FG)
        .bg(palette::DIALOG_BG);
    Clear.render(area, &mut buffer);
    Paragraph::new(lines(snapshot, text_width))
        .block(dialog_frame(title(snapshot.operation), "", ground))
        .style(ground)
        .render(area, &mut buffer);
    buffer
}

/// Centred, as wide as the screen allows up to [`MAX_WIDTH`].
fn dialog_area(screen: Rect) -> Rect {
    let width = screen.width.saturating_sub(2).min(MAX_WIDTH);
    let height = HEIGHT.min(screen.height);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

const fn title(operation: Operation) -> &'static str {
    match operation {
        Operation::Copy => " Copy ",
        Operation::Move => " Move ",
        Operation::Delete => " Delete ",
        Operation::Find => " Find ",
    }
}

fn lines(snapshot: &Snapshot, width: usize) -> Vec<Line<'static>> {
    let heading = match (snapshot.phase, snapshot.operation) {
        (_, Operation::Find) => "Searching in",
        (Phase::Scanning, _) => "Scanning",
        (Phase::Working, Operation::Copy) => "Copying",
        (Phase::Working, Operation::Move) => "Moving",
        (Phase::Working, Operation::Delete) => "Deleting",
    };
    let current = printable_name(&crate::path_display::label(&snapshot.current)).into_owned();
    let meter = match (snapshot.phase, snapshot.operation) {
        (Phase::Scanning, _) | (_, Operation::Find) => String::new(),
        (Phase::Working, _) => bar(snapshot.permille(), width),
    };
    vec![
        Line::from(heading),
        Line::from(tail(&current, width)),
        Line::from(""),
        Line::from(meter),
        Line::from(counts(snapshot)),
        Line::from(format!("Time: {}", clock(snapshot.elapsed.as_secs()))),
    ]
}

/// `█████░░░░░  42%`, as wide as `width`.
fn bar(permille: u16, width: usize) -> String {
    let percent = format!("{:>4}%", permille / 10);
    let cells = width.saturating_sub(percent.len() + 1);
    let filled = cells * usize::from(permille) / 1000;
    format!(
        "{}{} {percent}",
        "█".repeat(filled),
        "░".repeat(cells - filled)
    )
}

/// `Files: 12 of 300   Size: 1.20 MB of 3.00 GB`; while scanning, only what
/// was found so far.
fn counts(snapshot: &Snapshot) -> String {
    let (done, total) = (snapshot.done, snapshot.total);
    if snapshot.operation == Operation::Find {
        return format!("Found: {}   Checked: {}", done.files, total.files);
    }
    match snapshot.phase {
        Phase::Scanning if total.bytes > 0 => format!(
            "Files: {}   Size: {}",
            total.files,
            FileSize::from_bytes(total.bytes)
        ),
        Phase::Scanning => format!("Files: {}", total.files),
        Phase::Working if total.bytes > 0 => format!(
            "Files: {} of {}   Size: {} of {}",
            done.files,
            total.files,
            FileSize::from_bytes(done.bytes),
            FileSize::from_bytes(total.bytes)
        ),
        Phase::Working => format!("Files: {} of {}", done.files, total.files),
    }
}

/// `hh:mm:ss`.
fn clock(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// The end of `text` that fits in `width` cells, `…` standing for the rest:
/// of a long path, the file name at the end is the part worth reading.
fn tail(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let mut kept = Vec::new();
    let mut used = 1; // the ellipsis
    for ch in text.chars().rev() {
        let cells = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cells > width {
            break;
        }
        used += cells;
        kept.push(ch);
    }
    std::iter::once('…').chain(kept.into_iter().rev()).collect()
}

#[cfg(test)]
mod tests;
