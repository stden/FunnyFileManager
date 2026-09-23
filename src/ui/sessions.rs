//! Purpose: Draws a panel that has become `NetBox`'s stored-sessions list.
//!
//! The frame is the file panel's, so the two panels keep reading as a pair:
//! Far's title in the top border, the one column's heading under it, the
//! rows, and the info row above the bottom border. Reads the rows the panel
//! carries and nothing else — the store is `ops`' business. The helpers the
//! remote panel shares — the title, the border, the cursor, the info row —
//! are here too, since both panels are drawn the same way.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::cells::printable_name;
use super::panel::{pad_cells, visible_window};
use super::{far_block, palette, Cells, FAR_BLUE, PANEL_INFO_ROW};
use crate::models::{sessions_panel, Panel, PanelKind, SessionRow, SessionsPanel};

/// The column heading row under the top border — the row the file panel
/// spends on its column titles, so a click lands on the same row in both.
const HEADER_ROWS: u16 = 1;

pub(super) fn render(f: &mut Frame<'_>, panel: &mut Panel, area: Rect, is_active: bool) {
    let PanelKind::Sessions(sessions) = &mut panel.kind else {
        return;
    };
    let Some(body) = frame(
        f,
        area,
        sessions_panel::TITLE,
        sessions_panel::COLUMN_TITLE,
        is_active,
    ) else {
        return;
    };
    render_rows(f, sessions, body.rows, is_active);
    let count = sessions.session_count();
    let plural = if count == 1 { "" } else { "s" };
    render_info(f, &format!("{count} session{plural}"), body.info);
}

/// Where a `NetBox` panel's rows and its info line go inside its frame.
pub(super) struct Body {
    pub(super) rows: Rect,
    pub(super) info: Rect,
}

/// Draw the frame titled `title` with the column heading `heading` under
/// it; `None` when the panel is too short to hold a row.
pub(super) fn frame(
    f: &mut Frame<'_>,
    area: Rect,
    title: &str,
    heading: &str,
    is_active: bool,
) -> Option<Body> {
    let block = far_block()
        .title(netbox_title(title, is_active))
        .border_style(border(is_active))
        .style(Style::default().bg(FAR_BLUE));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height <= HEADER_ROWS + PANEL_INFO_ROW {
        return None;
    }
    let [heading_row, rows, info] = Layout::vertical([
        Constraint::Length(HEADER_ROWS),
        Constraint::Fill(1),
        Constraint::Length(PANEL_INFO_ROW),
    ])
    .areas(inner);
    render_heading(f, heading, heading_row);
    Some(Body { rows, info })
}

/// The column heading, on the first row inside the frame.
fn render_heading(f: &mut Frame<'_>, heading: &str, row: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            pad_cells(heading, Cells::columns(row.width)),
            Style::default().fg(palette::HOTKEY).bg(FAR_BLUE),
        ))),
        row,
    );
}

/// Far highlights the active panel's title; a `NetBox` panel's is its name.
fn netbox_title(text: &str, is_active: bool) -> Line<'static> {
    let style = if is_active {
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG)
    } else {
        Style::default().fg(palette::FRAME).bg(FAR_BLUE)
    };
    Line::from(Span::styled(format!(" {} ", printable_name(text)), style)).centered()
}

fn border(is_active: bool) -> Style {
    let border = Style::default().fg(palette::FRAME);
    if is_active {
        border.add_modifier(Modifier::BOLD)
    } else {
        border
    }
}

/// The rows on screen, scrolled so the cursor row is always drawn.
fn render_rows(f: &mut Frame<'_>, sessions: &mut SessionsPanel, body: Rect, is_active: bool) {
    let rows = sessions.rows();
    let window = visible_window(
        rows.len(),
        usize::from(body.height),
        sessions.scroll,
        Some(sessions.cursor()),
    );
    let width = Cells::columns(body.width);
    let lines: Vec<Line<'_>> = rows
        .get(window.clone())
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(offset, row)| {
            let chosen = is_active && window.start + offset == sessions.cursor();
            Line::from(Span::styled(
                pad_cells(&row_text(row), width),
                row_style(row, chosen),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), body);
    sessions.scroll = window.start;
}

/// The name, and `user@host` after it when the name is not already that.
fn row_text(row: &SessionRow) -> String {
    match row {
        SessionRow::Parent => sessions_panel::PARENT.to_owned(),
        SessionRow::Session {
            name,
            login: Some(login),
        } => printable_name(&format!("{name}  {login}")).into_owned(),
        SessionRow::Session { name, login: None } => printable_name(name.as_str()).into_owned(),
    }
}

pub(super) fn cursor_style() -> Style {
    Style::default()
        .bg(palette::CURSOR_BG)
        .fg(palette::CURSOR_FG)
}

fn row_style(row: &SessionRow, chosen: bool) -> Style {
    if chosen {
        return cursor_style();
    }
    match row {
        SessionRow::Parent => Style::default()
            .fg(palette::DIRECTORY)
            .add_modifier(Modifier::BOLD),
        SessionRow::Session { .. } => Style::default().fg(palette::FILE),
    }
}

/// The info row above the bottom border.
pub(super) fn render_info(f: &mut Frame<'_>, text: &str, row: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            pad_cells(&printable_name(text), Cells::columns(row.width)),
            Style::default().fg(palette::DATA).bg(FAR_BLUE),
        ))),
        row,
    );
}
