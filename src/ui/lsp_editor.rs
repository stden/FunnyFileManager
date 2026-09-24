//! Purpose: Draw the Language servers dialog — the server list with the
//! selected server's detail, the F3 view of every server, the edit form and
//! the exit question — on the panel's blue, with its own key bar.
//!
//! Full screen, like the viewer and the editor, because the code colours are
//! designed to be read on that ground. Rendering records the sizes it drew
//! (`LspEditor::record_drawn`) so paging keys move by what the user saw.

use std::collections::BTreeSet;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use super::fkey_bar::{render_key_bar, KeyLabels};
use super::{far_block, fit_name, head_cells, palette, Cells, FAR_BLUE};
use crate::config::LanguageId;
use crate::models::lsp_editor::{Access, Drawn, Mode, Notice, ReadOnlyReason};
use crate::models::LspEditor;

mod detail;
mod form;
mod list;

/// Terminals shorter than this lose the detail pane; F3 still shows it.
const DETAIL_MIN_ROWS: u16 = 16;

/// Whether the list screen has room for the selected server's detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailPane {
    Shown,
    Hidden,
}

/// The window's rows inside its frame, top to bottom.
struct Rows {
    settings: Rect,
    list: Rect,
    detail: Option<Rect>,
    message: Rect,
}

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

/// The framed window above the key bar, and the key bar.
fn split(screen: Rect) -> Option<(Rect, Rect)> {
    if screen.height < 3 || screen.width < 4 {
        return None;
    }
    let window = Rect {
        height: screen.height - 1,
        ..screen
    };
    let bar = Rect {
        y: screen.y + screen.height - 1,
        height: 1,
        ..screen
    };
    Some((window, bar))
}

/// The window's frame, with `title` on top and `legend` in the bottom border.
fn frame(title: &str, legend: &str) -> Block<'static> {
    far_block()
        .title(Line::from(format!(" {title} ")).centered())
        .title_bottom(Line::from(format!(" {legend} ")))
        .border_style(Style::default().fg(palette::FRAME))
        .style(ground())
}

/// Inside the frame, one blank column each side.
fn inner(window: Rect) -> Rect {
    Rect {
        x: window.x + 2,
        y: window.y + 1,
        width: window.width.saturating_sub(4),
        height: window.height.saturating_sub(2),
    }
}

/// A `╟───╢` rule across the window at `y`, when `y` is above `below`, the
/// first row that belongs to something else.
fn rule(f: &mut Frame<'_>, window: Rect, y: u16, below: u16) {
    if y >= below {
        return;
    }
    let line = format!(
        "╟{}╢",
        "─".repeat(usize::from(window.width.saturating_sub(2)))
    );
    f.render_widget(
        Paragraph::new(line).style(Style::default().fg(palette::FRAME).bg(FAR_BLUE)),
        Rect {
            y,
            height: 1,
            ..window
        },
    );
}

fn line_at(f: &mut Frame<'_>, area: Rect, y: u16, line: Line<'static>) {
    if y < area.y + area.height {
        f.render_widget(
            Paragraph::new(line).style(ground()),
            Rect {
                y,
                height: 1,
                ..area
            },
        );
    }
}

/// The rows of the list screen for a window: the detail pane only on a
/// terminal tall enough, the list at most half of what is left. A window too
/// short for them all keeps the settings row, then the message row under
/// it; a row that does not fit has no height.
fn rows(window: Rect, servers: usize, screen_rows: u16) -> Rows {
    let area = inner(window);
    let with_detail = screen_rows >= DETAIL_MIN_ROWS;
    let chrome = if with_detail { 5 } else { 4 };
    let body = area.height.saturating_sub(chrome);
    let wanted = u16::try_from(servers).unwrap_or(u16::MAX);
    let list_height = if with_detail {
        wanted.min(body / 2)
    } else {
        wanted.min(body)
    };
    let row = |y: u16, height: u16| Rect { y, height, ..area };
    let list = row(area.y + 2, list_height);
    let detail_y = list.y + list_height + 1;
    let detail = with_detail.then(|| row(detail_y, body - list_height));
    // Below two rows there is no message row, and no row for a rule either.
    let message = if area.height > 1 {
        row(area.y + area.height - 1, 1)
    } else {
        row(area.y, 0)
    };
    Rows {
        settings: row(area.y, area.height.min(1)),
        list,
        detail,
        message,
    }
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
            Style::default().fg(palette::DISABLED_FG),
        )));
    }
    f.render_widget(Paragraph::new(lines).style(ground()), area);
}

/// Row 1: the file F2 writes, or why there is none.
fn settings_line(editor: &LspEditor, width: u16) -> Line<'static> {
    let path = |path: &std::path::Path| {
        Span::styled(
            path.display().to_string(),
            super::code::token_style(crate::highlight::Token::Str),
        )
    };
    let failed = Style::default().fg(palette::STATUS_FAILED);
    let mut spans = vec![Span::raw("Settings: ")];
    match editor.access() {
        Access::Writable { path: file } => spans.push(path(file)),
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile) => {
            spans.push(Span::styled("none for this session — read only", failed));
        }
        Access::ReadOnly(reason @ ReadOnlyReason::Document { path: file, .. }) => {
            spans.push(path(file));
            spans.push(Span::styled(format!(" — read only: {reason}"), failed));
        }
    }
    fit_line(spans, width)
}

/// The message row: the notice the last key set, else why the dialog is
/// read-only, else what changed.
fn message_line(editor: &LspEditor, pane: DetailPane, width: u16) -> Line<'static> {
    let notice = editor
        .notice()
        .cloned()
        .or_else(|| editor.read_only_notice());
    let (text, color) = match (notice, pane) {
        (Some(Notice::Info(text)), _) => (text, palette::CODE_NORMAL),
        (Some(Notice::Error(text)), _) => (text, palette::STATUS_FAILED),
        (None, DetailPane::Hidden) => ("F3 shows details".to_owned(), palette::CODE_NORMAL),
        (None, DetailPane::Shown) => (changes(editor), palette::CODE_NORMAL),
    };
    fit_line(vec![Span::styled(text, Style::default().fg(color))], width)
}

/// `Modified: python, rust. F2 saves only what changed.`, or `No changes`.
fn changes(editor: &LspEditor) -> String {
    let changed: BTreeSet<&str> = editor
        .draft()
        .0
        .keys()
        .chain(editor.baseline().0.keys())
        .map(String::as_str)
        .filter(|key| editor.is_changed(&LanguageId::from_key(key)))
        .collect();
    if changed.is_empty() {
        return "No changes".to_owned();
    }
    let changed: Vec<&str> = changed.into_iter().collect();
    format!(
        "Modified: {}. F2 saves only what changed.",
        changed.join(", ")
    )
}

/// `spans` cut to `width` cells, the last one marked `…` when cut.
fn fit_line(spans: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let mut room = Cells::columns(width);
    let mut kept = Vec::new();
    for span in spans {
        let cells = Cells::of(&span.content);
        if cells <= room {
            room = room.saturating_sub(cells);
            kept.push(span);
            continue;
        }
        kept.push(Span::styled(head_cells(&span.content, room), span.style));
        break;
    }
    Line::from(kept)
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

/// The list's key bar; a read-only dialog blanks the keys that change it.
const LIST_KEYS: KeyLabels<'static> = [
    ("1", ""),
    ("2", "Save"),
    ("3", "View"),
    ("4", "Edit"),
    ("5", "Check"),
    ("6", ""),
    ("7", "Add"),
    ("8", "Reset"),
    ("9", ""),
    ("10", "Quit"),
    ("11", ""),
    ("12", ""),
];

/// The F3 view's key bar.
const VIEW_KEYS: KeyLabels<'static> = [
    ("1", ""),
    ("2", ""),
    ("3", "List"),
    ("4", ""),
    ("5", "Check"),
    ("6", ""),
    ("7", ""),
    ("8", ""),
    ("9", ""),
    ("10", "Back"),
    ("11", ""),
    ("12", ""),
];

/// The keys that are not F-keys, written into the bottom border, for what
/// is on screen; the F3 view writes its own.
fn legend(editor: &LspEditor) -> String {
    match editor.mode() {
        Mode::Form(dialog) => form::legend(dialog).to_owned(),
        Mode::ConfirmExit(_) => form::EXIT_LEGEND.to_owned(),
        Mode::Browse | Mode::View(_) => list::legend(editor),
    }
}

/// The dialog's own key bar: a label only where the key does something.
fn key_labels(editor: &LspEditor) -> KeyLabels<'static> {
    match editor.mode() {
        Mode::View(_) => VIEW_KEYS,
        // No F-key acts in the form or the exit question.
        Mode::Form(_) | Mode::ConfirmExit(_) => LIST_KEYS.map(|(key, _)| (key, "")),
        Mode::Browse => list_key_labels(editor),
    }
}

/// The list's key bar: Del's label follows the selected server, and a
/// read-only dialog blanks the keys that change the table.
fn list_key_labels(editor: &LspEditor) -> KeyLabels<'static> {
    let mut labels = LIST_KEYS;
    if list::removes(editor) {
        relabel(&mut labels, "8", "Remove");
    }
    if matches!(editor.access(), Access::ReadOnly(_)) {
        for key in ["2", "4", "7", "8"] {
            relabel(&mut labels, key, "");
        }
    }
    labels
}

fn relabel(labels: &mut KeyLabels<'static>, key: &str, label: &'static str) {
    if let Some(cell) = labels.iter_mut().find(|(number, _)| *number == key) {
        cell.1 = label;
    }
}

/// A fixed-width column, cut with `…` and padded.
fn column(text: &str, width: usize) -> String {
    fit_name(text, Cells::new(width))
}

#[cfg(test)]
mod tests;
