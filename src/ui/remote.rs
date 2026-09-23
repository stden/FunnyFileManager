//! Purpose: Draws a panel that browses a remote host through a session.
//!
//! The frame is the sessions panel's, with the session and the remote
//! directory in the title where the local path would be, a `Name` heading,
//! the entries as rows, and the directory in the info row. A name is data
//! from the host, so every character a terminal would not draw is shown as
//! a mark rather than obeyed.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::cells::printable_name;
use super::panel::{pad_cells, visible_window};
use super::sessions::{cursor_style, frame, render_info};
use super::{palette, Cells};
use crate::models::{DirEntry, Panel, PanelKind, RemotePanel};

/// The one column's heading.
const COLUMN_TITLE: &str = "Name";

pub(super) fn render(f: &mut Frame, panel: &mut Panel, area: Rect, is_active: bool) {
    let PanelKind::Remote(remote) = &mut panel.kind else {
        return;
    };
    let Some(body) = frame(f, area, &remote.title(), COLUMN_TITLE, is_active) else {
        return;
    };
    render_rows(f, remote, body.rows, is_active);
    render_info(f, remote.path().as_str(), body.info);
}

/// The rows on screen, scrolled so the cursor row is always drawn.
fn render_rows(f: &mut Frame, remote: &mut RemotePanel, body: Rect, is_active: bool) {
    let entries = remote.entries();
    let window = visible_window(
        entries.len(),
        usize::from(body.height),
        remote.scroll,
        Some(remote.cursor()),
    );
    let width = Cells::columns(body.width);
    let lines: Vec<Line> = entries[window.clone()]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let chosen = is_active && window.start + offset == remote.cursor();
            Line::from(Span::styled(
                pad_cells(&printable_name(&entry.name), width),
                row_style(entry, chosen),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), body);
    remote.scroll = window.start;
}

fn row_style(entry: &DirEntry, chosen: bool) -> Style {
    if chosen {
        cursor_style()
    } else if entry.kind.is_dir() {
        Style::default()
            .fg(palette::DIRECTORY)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette::FILE)
    }
}
