//! Purpose: Draws a panel that has become a folder tree (`Ctrl+T`).
//!
//! The frame is the file panel's, so the two panels keep reading as a pair;
//! inside it are Far's rows — `├─` and `└─` hanging off `│` guide lines —
//! and the info row at the bottom names the folder under the cursor, or the
//! search being typed. Reads the tree the panel carries and nothing else —
//! the disk is `ops`' business.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use super::panel::visible_window;
use super::{far_block, palette, Cells, FAR_BLUE, PANEL_INFO_ROW};
use crate::models::{Branch, FolderTree, Guide, Panel, PanelKind, TreeRow};

pub(super) fn render(f: &mut Frame, panel: &mut Panel, area: Rect, is_active: bool) {
    let PanelKind::Tree(tree) = &mut panel.kind else {
        return;
    };
    let border = Style::default().fg(palette::FRAME);
    let block = far_block()
        .title(title(is_active))
        .border_style(if is_active {
            border.add_modifier(Modifier::BOLD)
        } else {
            border
        })
        .style(Style::default().bg(FAR_BLUE));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height <= PANEL_INFO_ROW {
        return;
    }
    let body = Rect {
        height: inner.height - PANEL_INFO_ROW,
        ..inner
    };
    render_rows(f, tree, body, is_active);
    render_info(f, tree, inner);
}

/// Far highlights the active panel's title; the tree has no path to show
/// there — that is the info row's job — so the title is the panel's kind.
fn title(is_active: bool) -> Line<'static> {
    let style = if is_active {
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG)
    } else {
        Style::default().fg(palette::FRAME).bg(FAR_BLUE)
    };
    Line::from(Span::styled(" Tree ", style)).centered()
}

/// Only the rows on screen are built, and the offset survives between frames
/// the way the file panel's does: the window holds still until the cursor
/// leaves it, then moves by just enough.
fn render_rows(f: &mut Frame, tree: &mut FolderTree, body: Rect, is_active: bool) {
    let window = visible_window(
        tree.rows().len(),
        usize::from(body.height),
        tree.scroll,
        Some(tree.cursor),
    );
    tree.scroll = window.start;
    let items: Vec<ListItem> = tree.rows()[window.clone()]
        .iter()
        .map(|row| ListItem::new(row_line(row)))
        .collect();
    let mut list = List::new(items).style(Style::default().bg(FAR_BLUE));
    // No cursor on the inactive panel: Far gives the screen one answer to
    // "what does Enter act on".
    if is_active {
        list = list.highlight_style(
            Style::default()
                .bg(palette::CURSOR_BG)
                .fg(palette::CURSOR_FG)
                .add_modifier(Modifier::BOLD),
        );
    }
    let mut state = ListState::default();
    state.select(tree.cursor.checked_sub(window.start));
    f.render_stateful_widget(list, body, &mut state);
}

/// The guide lines in the frame colour, the name in the directory colour —
/// the same white the file panel gives a folder, so a name means the same
/// thing on both panels.
fn row_line(row: &TreeRow) -> Line<'static> {
    Line::from(vec![
        Span::styled(guide_text(row), Style::default().fg(palette::FRAME)),
        Span::styled(row.name(), Style::default().fg(palette::DIRECTORY)),
    ])
}

/// Far's connectors: one column pair per ancestor, then the row's own hook.
fn guide_text(row: &TreeRow) -> String {
    let mut text = String::new();
    for guide in &row.guides {
        text.push_str(match guide {
            Guide::Line => "\u{2502} ",
            Guide::Gap => "  ",
        });
    }
    text.push_str(match row.branch {
        Branch::Root => "",
        Branch::Middle => "\u{251c}\u{2500}",
        Branch::Last => "\u{2514}\u{2500}",
    });
    text
}

/// The row above the bottom border: the search being typed, else the full
/// path of the folder under the cursor — which is what the other panel is
/// showing, or would show on Enter.
fn render_info(f: &mut Frame, tree: &FolderTree, inner: Rect) {
    let text = tree.search.query().map_or_else(
        || {
            tree.selected()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        },
        |query| format!("Search: {query}_"),
    );
    let width = Cells::columns(inner.width);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            super::head_cells(&text, width),
            Style::default().fg(Color::LightCyan).bg(FAR_BLUE),
        ))),
        Rect {
            y: inner.y + inner.height - PANEL_INFO_ROW,
            height: PANEL_INFO_ROW,
            ..inner
        },
    );
}

#[cfg(test)]
mod tests;
