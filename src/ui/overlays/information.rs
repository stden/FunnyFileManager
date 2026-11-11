//! Purpose: Far's Ctrl+L information dialog: the folder's totals, then where
//! the space is (largest folders) and what it is (by content, largest
//! files), or why the folder could not be read.

use ratatui::Frame;

use super::super::{dialog, head_cells, Cells};

/// What one line of the information panel is.
enum Info {
    Summary(String),
    Heading(&'static str),
    Detail(String),
}

/// Far's Ctrl+L panel, answering "where is the space and what is all this".
///
/// Three sections because they answer three different questions: the totals say
/// how much there is, the folders say *where* it is, and the categories say
/// *what* it is. One flat list would answer none of them.
pub(super) fn render_information(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    stats: &crate::stats::DirectoryStats,
) {
    let lines = information_lines(stats);
    // A heading opens a new group, so it is preceded by the rule that divides
    // one from the last — except the first, which follows the summary's own.
    let mut rows = Vec::new();
    for line in &lines {
        match line {
            Info::Summary(text) => rows.push(dialog::Row::Text(text)),
            Info::Heading(title) => {
                rows.push(dialog::Row::Divider);
                rows.push(dialog::Row::Label(title));
            }
            Info::Detail(text) => rows.push(dialog::Row::Label(text)),
        }
    }

    dialog::render_tracked(
        f,
        tracker,
        "Information",
        dialog::Ground::Prompt,
        &rows,
        &[dialog::Button::new("&Ok").default_button().focused(true)],
    );
}

fn information_lines(stats: &crate::stats::DirectoryStats) -> Vec<Info> {
    use super::super::panel::abbreviate_bytes;
    use crate::stats::Coverage;

    // No totals at all for a folder that could not be read: its zeros would
    // read as an empty folder.
    if let Coverage::Unreadable(reason) = &stats.coverage {
        let reason = crate::ui::printable_name(reason);
        return vec![Info::Summary(format!("Cannot read this folder: {reason}"))];
    }
    let mut lines = vec![Info::Summary(format!(
        "{} in {} files, {} folders",
        abbreviate_bytes(stats.total.bytes()),
        stats.files,
        stats.directories
    ))];
    match stats.coverage {
        Coverage::Partial => lines.push(Info::Summary(
            "measured to the walk limit — totals are lower bounds".to_owned(),
        )),
        Coverage::Unread(folders) => lines.push(Info::Summary(format!(
            "{folders} folder(s) could not be read — totals are lower bounds"
        ))),
        Coverage::Complete | Coverage::Unreadable(_) => {}
    }

    append_information_sections(&mut lines, stats);

    lines
}

fn append_information_sections(lines: &mut Vec<Info>, stats: &crate::stats::DirectoryStats) {
    if !stats.children.is_empty() {
        lines.push(Info::Heading("Largest folders"));
        lines.extend(stats.children.iter().map(|child| {
            Info::Detail(information_entry(
                &child.name,
                child.size.bytes(),
                Some(child.files),
            ))
        }));
    }
    if !stats.by_kind.is_empty() {
        lines.push(Info::Heading("By content"));
        lines.extend(stats.by_kind.iter().map(|kind| {
            Info::Detail(information_entry(
                kind.kind.label(),
                kind.size.bytes(),
                Some(kind.files),
            ))
        }));
    }
    if !stats.largest.is_empty() {
        lines.push(Info::Heading("Largest files"));
        lines.extend(
            stats
                .largest
                .iter()
                .map(|file| Info::Detail(information_entry(&file.path, file.size.bytes(), None))),
        );
    }
}

/// Columns given to a name before its size.
fn information_entry(name: &str, size: u64, count: Option<usize>) -> String {
    use super::super::panel::abbreviate_bytes;
    const NAME: usize = 30;
    let clean = crate::ui::printable_name(name);
    let name = head_cells(&clean, Cells::new(NAME));
    let padding = " ".repeat(NAME.saturating_sub(Cells::of(&name).get()));
    match count {
        Some(files) => format!("{name}{padding} {:>9}  {files}", abbreviate_bytes(size)),
        None => format!("{name}{padding} {:>9}", abbreviate_bytes(size)),
    }
}
