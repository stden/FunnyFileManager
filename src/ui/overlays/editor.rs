//! Purpose: The built-in F4 editor, full screen as Far's is: the bar with the
//! file and the cursor's line, column and code point, the body scrolled to
//! keep the caret in view, and the caret placed in cells.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::code::editor_window;
use super::super::{head_cells, palette, screen_u16, Cells, EditorLine, FAR_BLUE};
use super::editor_dialogs::render_editor_dialog;

/// Rows the editor spends on chrome: its top bar, and nothing else.
const EDITOR_BAR_HEIGHT: u16 = 1;

/// Far's editor bar: the file on the left, where the cursor is on the right.
///
/// `Ch` is the code point under the cursor, which is the reason the bar is
/// worth its row — it is the one fact about a file you cannot get by looking
/// at it, and the one you need when a file will not parse.
fn render_editor_bar(f: &mut Frame<'_>, editor: &crate::models::Editor, area: Rect, line: &str) {
    let text = editor_bar_text(editor, area.width, line);
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(text))).style(
            Style::default()
                .bg(palette::STATUS_BAR_BG)
                .fg(palette::STATUS_BAR_FG),
        ),
        Rect {
            height: EDITOR_BAR_HEIGHT,
            ..area
        },
    );
}

fn editor_bar_text(editor: &crate::models::Editor, width: u16, line: &str) -> String {
    let marker = if editor.state.is_modified() { " *" } else { "" };
    let column = editor.cursor_column().chars();
    let code = line.chars().nth(column).map_or(0, u32::from);
    let position = format!(
        " Ln {}/{}  Col {}  Ch {code:#06x} ",
        editor.cursor_line + 1,
        editor.lines.len(),
        column + 1,
    );

    let width = Cells::columns(width);
    let label = editor.notice.as_ref().map_or_else(
        || super::super::editor_lsp::label(editor, marker),
        |notice| format!(" {notice}"),
    );
    // An operation's result must remain visible on a narrow terminal. Keep
    // the cursor details alongside it only when both fit in the header.
    let position = if editor.notice.is_some() && Cells::of(&label) + Cells::of(&position) > width {
        String::new()
    } else {
        position
    };
    let name = head_cells(&label, width.saturating_sub(Cells::of(&position)));
    let gap = width
        .saturating_sub(Cells::of(&name) + Cells::of(&position))
        .get();

    format!("{name}{}{position}", " ".repeat(gap))
}

pub(super) fn render_editor(
    f: &mut Frame<'_>,
    editor: &crate::models::Editor,
    tracker: &mut crate::mouse::DialogMouse,
) {
    // Full screen, as Far's editor is. A window inset inside the panels
    // leaves the columns showing round the edges, which reads as a
    // dialog floating over the file manager rather than as the editor
    // you switched into — and it costs the file ten columns of width.
    let area = f.area();
    // One row of chrome, not a frame. Far's editor spends its top row
    // on the file name and the cursor position and gives every other
    // cell to the file — a box would cost two rows and two columns to
    // draw a line nobody reads.
    let inner_h = usize::from(area.height.saturating_sub(EDITOR_BAR_HEIGHT));
    let inner_w = Cells::columns(area.width);

    // Stateless scrolling: keep the cursor roughly centered vertically
    // and always visible horizontally — no scroll state to desync.
    let scroll_y = editor.cursor_line.saturating_sub(inner_h / 2);

    // `cursor_col` is a CHAR index into the buffer, which is what
    // editing needs — but the screen budgets in CELLS, and a CJK glyph
    // spends two. Scrolling and positioning by the char index put both
    // the window and the cursor in the wrong column on any non-ASCII
    // line; convert once, here, and work in cells below.
    let cursor_line_text = editor
        .lines
        .get(editor.cursor_line)
        .map_or("", String::as_str);
    let cursor_line = EditorLine {
        text: cursor_line_text,
        tab: editor.tab_size,
    };
    let cursor_cell = cursor_line.cells_before(editor.cursor_column());
    let scroll_x = cursor_cell.saturating_sub(inner_w.saturating_sub(Cells::new(1)));

    render_editor_bar(f, editor, area, cursor_line_text);

    render_editor_body(f, editor, area, scroll_y, scroll_x);

    // Position against the cells actually dropped, not against `scroll_x`:
    // a double-width glyph straddling the cut is dropped whole.
    let dropped = cursor_line.dropped(scroll_x);
    let caret = (
        area.x
            .saturating_add(screen_u16(cursor_cell.saturating_sub(dropped).get())),
        (area.y + EDITOR_BAR_HEIGHT).saturating_add(screen_u16(editor.cursor_line - scroll_y)),
    );
    if !render_editor_dialog(f, tracker, editor, caret) {
        f.set_cursor_position(caret);
    }
}

fn render_editor_body(
    f: &mut Frame<'_>,
    editor: &crate::models::Editor,
    area: Rect,
    scroll_y: usize,
    scroll_x: Cells,
) {
    let body_area = Rect {
        y: area.y + EDITOR_BAR_HEIGHT,
        height: area.height.saturating_sub(EDITOR_BAR_HEIGHT),
        ..area
    };
    let body = editor_window(
        editor,
        scroll_y,
        usize::from(body_area.height),
        scroll_x,
        Cells::columns(body_area.width),
    );
    f.render_widget(Clear, body_area);
    f.render_widget(
        Paragraph::new(body).style(Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)),
        body_area,
    );
    super::super::editor_selection::render(f, editor, body_area, scroll_y, scroll_x);
    super::super::editor_lsp::diagnostics(f, editor, body_area, scroll_y, scroll_x);
    super::super::editor_brackets::render(f, editor, body_area, scroll_y, scroll_x);
}
