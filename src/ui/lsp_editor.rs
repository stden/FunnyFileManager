//! Purpose: Draw the Language servers dialog — the server list with the
//! selected server's detail, the F3 view of every server, the edit form and
//! the exit question — on the panel's blue, with its own key bar.
//!
//! Full screen, like the viewer and the editor, because the code colours are
//! designed to be read on that ground. Rendering records the sizes it drew
//! (`LspEditor::record_drawn`) so paging keys move by what the user saw.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use super::fkey_bar::render_key_bar;
use super::{fit_name, palette, Cells, FAR_BLUE};
use crate::models::lsp_editor::{Drawn, Mode};
use crate::models::LspEditor;
use keys::{key_labels, legend};
use lines::{fit_line, message_line, settings_line};
use window::{frame, inner, line_at, rows, rule, split, DetailPane};

mod detail;
mod form;
mod keys;
mod lines;
mod list;
mod window;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    editor: &mut LspEditor,
) {
    let screen = f.area();
    f.render_widget(Clear, screen);
    f.render_widget(Block::default().style(ground()), screen);
    let Some((window, bar)) = split(screen) else {
        tracker.clear();
        return;
    };
    render_key_bar(f, bar, &key_labels(editor));
    if matches!(editor.mode(), Mode::View(_)) {
        render_view(f, window, editor);
    } else {
        render_browse(f, window, editor);
    }
    match editor.mode() {
        Mode::Form(dialog) => form::render(f, tracker, dialog),
        Mode::ConfirmExit(choice) => form::render_exit_question(f, tracker, *choice),
        Mode::Browse | Mode::View(_) => tracker.clear(),
    }
}

fn ground() -> Style {
    Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)
}

fn render_browse(f: &mut Frame<'_>, window: Rect, editor: &mut LspEditor) {
    let rows = rows(window, editor.draft().0.len(), f.area().height);
    f.render_widget(frame("Language servers", &legend(editor)), window);
    line_at(
        f,
        rows.settings,
        rows.settings.y,
        settings_line(editor, rows.settings.width),
    );
    // The rules give way to the message row on a short window.
    let below = rows.message.y;
    rule(f, window, rows.settings.y + 1, below);
    list::render(f, rows.list, editor);
    rule(f, window, rows.list.y + rows.list.height, below);
    if let Some(detail) = rows.detail {
        render_detail(f, detail, editor);
        rule(f, window, detail.y + detail.height, below);
    }
    let pane = if rows.detail.is_some() {
        DetailPane::Shown
    } else {
        DetailPane::Hidden
    };
    let message = message_line(editor, pane, rows.message.width);
    line_at(f, rows.message, rows.message.y, message);
    editor.record_drawn(Drawn::List {
        page: usize::from(rows.list.height),
    });
}

/// The selected server's detail; when it does not fit, the last row says
/// where the rest is.
fn render_detail(f: &mut Frame<'_>, area: Rect, editor: &LspEditor) {
    let Some((language, server)) = editor.selected_server() else {
        return;
    };
    let mut lines = detail::detail_lines(editor, language, server, Cells::columns(area.width));
    let height = usize::from(area.height);
    if lines.len() > height {
        lines.truncate(height.saturating_sub(1));
        lines.push(Line::from(Span::styled(
            "… F3 shows everything",
            Style::default().fg(palette::HIDDEN),
        )));
    }
    f.render_widget(Paragraph::new(lines).style(ground()), area);
}

/// F3: every server, scrolled, with where the view is in the bottom border.
fn render_view(f: &mut Frame<'_>, window: Rect, editor: &mut LspEditor) {
    let area = inner(window);
    let content = Rect {
        y: area.y + 3,
        height: area.height.saturating_sub(3),
        ..area
    };
    let lines = detail::every_server(editor, Cells::columns(content.width));
    let page = usize::from(content.height);
    editor.record_drawn(Drawn::View {
        lines: lines.len(),
        page,
    });
    let top = match editor.mode() {
        Mode::View(top) => top.lines(),
        Mode::Browse | Mode::Form(_) | Mode::ConfirmExit(_) => 0,
    };
    let legend = view_legend(top, page, lines.len());
    f.render_widget(frame("Language servers: all", &legend), window);
    line_at(f, area, area.y, settings_line(editor, area.width));
    let note = "Availability is what FFM would start; it is not a process list.";
    line_at(
        f,
        area,
        area.y + 1,
        fit_line(vec![Span::raw(note)], area.width),
    );
    rule(f, window, area.y + 2, area.y + area.height);
    let shown: Vec<Line<'_>> = lines.into_iter().skip(top).take(page).collect();
    f.render_widget(Paragraph::new(shown).style(ground()), content);
}

/// Where the view is, and its keys.
fn view_legend(top: usize, page: usize, total: usize) -> String {
    let last = (top + page).min(total);
    let first = (top + 1).min(last);
    format!("Lines {first}-{last} of {total}  Up Down PgUp PgDn Home End  F3/Esc List")
}

/// A fixed-width column, cut with `…` and padded.
fn column(text: &str, width: usize) -> String {
    fit_name(text, Cells::new(width))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_detail_edges;
