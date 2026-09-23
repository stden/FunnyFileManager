//! Purpose: Draws a panel that has become a quick view (`Ctrl+Q`).
//!
//! The frame is the file panel's, so the two panels keep reading as a pair:
//! the previewed path in the title where the directory would be, the text
//! where the entries would be, and the rule and info row above the bottom
//! border where the file panel keeps its own. Reads the preview the panel
//! carries and nothing else — the disk is `ops`' business.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::code::highlighted_window;
use super::panel::abbreviate_bytes;
use super::{far_block, fit_name, head_cells, palette, Cells, FAR_BLUE};
use crate::highlight::Highlighter;
use crate::models::{Panel, PanelKind, Preview, QuickView, TextPreview};
use crate::stats::{Coverage, DirectoryStats};

/// The rule above the info row, and the info row: what the frame keeps for
/// itself below the text.
const FOOTER_ROWS: u16 = 2;

pub(super) fn render(f: &mut Frame, panel: &Panel, area: Rect, is_active: bool) {
    let PanelKind::QuickView(view) = &panel.kind else {
        return;
    };
    let border = Style::default().fg(palette::FRAME);
    let border = if is_active {
        border.add_modifier(Modifier::BOLD)
    } else {
        border
    };
    let block = far_block()
        .title(Line::from(title(view, area.width)).centered())
        .border_style(border)
        .style(Style::default().bg(palette::BACKGROUND));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < FOOTER_ROWS.saturating_add(1) {
        render_body(f, view, inner);
        return;
    }
    render_body(
        f,
        view,
        Rect {
            height: inner.height - FOOTER_ROWS,
            ..inner
        },
    );
    render_footer(f, view, area, inner, border);
}

/// The previewed path, or the panel's name before there is one.
fn title(view: &QuickView, width: u16) -> String {
    let text = view.shown.as_ref().map_or_else(
        || "Quick view".to_owned(),
        |path| crate::path_display::label(path).into_owned(),
    );
    format!(
        " {} ",
        fit_name(&text, Cells::columns(width.saturating_sub(4)))
    )
}

fn render_body(f: &mut Frame, view: &QuickView, body: Rect) {
    let lines = match &view.preview {
        Preview::Nothing => Vec::new(),
        Preview::Text(text) => text_lines(view, text, body),
        Preview::Folder(stats) => folder_lines(stats),
        Preview::Refused(refusal) => vec![Line::from(refusal.label())],
    };
    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)),
        body,
    );
}

/// The visible window of the file, coloured as the viewer would colour it.
fn text_lines(view: &QuickView, text: &TextPreview, body: Rect) -> Vec<Line<'static>> {
    highlighted_window(
        Highlighter::for_file(&text.path, &text.lines),
        &text.lines,
        view.scroll.lines(),
        usize::from(body.height),
        Cells::new(view.col.cells()),
        Cells::columns(body.width),
    )
}

/// Far's folder answer: totals, not a listing.
fn folder_lines(stats: &DirectoryStats) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "Folder",
            Style::default()
                .fg(palette::DIRECTORY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "Total size:  {}",
            abbreviate_bytes(stats.total.bytes())
        )),
        Line::from(format!("Files:       {}", stats.files)),
        Line::from(format!("Folders:     {}", stats.directories)),
    ];
    if stats.coverage == Coverage::Partial {
        lines.push(Line::from("(walk limit reached: lower bounds)"));
    }
    lines
}

/// The rule joined to the frame, then the info row: what the viewer's status
/// line would say about the file, in the row the file panel spends on the
/// entry under its cursor.
fn render_footer(f: &mut Frame, view: &QuickView, area: Rect, inner: Rect, border: Style) {
    let rule = format!("╟{}╢", "─".repeat(usize::from(inner.width)));
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(rule, border))).style(Style::default().bg(FAR_BLUE)),
        Rect {
            y: inner.y + inner.height - FOOTER_ROWS,
            height: 1,
            ..area
        },
    );
    let text = info_row(view, usize::from(inner.height - FOOTER_ROWS));
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            head_cells(&text, Cells::columns(inner.width)),
            Style::default().fg(palette::FILE).bg(FAR_BLUE),
        ))),
        Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        },
    );
}

fn info_row(view: &QuickView, page: usize) -> String {
    match &view.preview {
        Preview::Text(text) => format!(
            "{} │ {} bytes │ {}%",
            text.encoding.label(),
            text.size.bytes(),
            super::scroll_percent(view.scroll.lines(), page, text.lines.len())
        ),
        Preview::Folder(stats) => format!("{} files, {} folders", stats.files, stats.directories),
        Preview::Nothing | Preview::Refused(_) => String::new(),
    }
}

#[cfg(test)]
mod tests;
