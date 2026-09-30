//! Purpose: A dialog's body inside its frame: each row drawn by what it is,
//! the caret of the focused text field, and the double rules that divide
//! the groups and set off the buttons.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{fit_quoting_path, palette, Cells};
use super::controls::{
    captioned_box, checkbox_line, field_caret, hotkey_spans, input_line, radio_line, FieldLook,
};
use super::row::Row;
use crate::status::PathMessage;

pub(super) fn render_body(
    f: &mut Frame<'_>,
    rows: &[Row<'_>],
    style: Style,
    body: Rect,
    framed: Rect,
) -> Option<(u16, u16)> {
    let mut caret = None;
    for (index, row) in rows.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        if offset >= body.height.saturating_sub(2) {
            break;
        }
        let y = body.y + offset;
        // One past the last character: the caret sits where the next one will
        // go, which is where a text field's cursor belongs.
        let column = row_caret(row);
        if let Some(column) = column {
            caret = Some((body.x + column, y));
        }
        render_row(
            f,
            row,
            style,
            body.width,
            Rect {
                y,
                height: 1,
                ..body
            },
            framed,
        );
    }

    caret
}

fn row_caret(row: &Row<'_>) -> Option<u16> {
    match row {
        Row::Input {
            value,
            focused: true,
            edit,
            ..
        } => {
            let before = edit.map_or(*value, |edit| edit.before_caret(value));
            u16::try_from(Cells::of(before).get()).ok()
        }
        Row::Field {
            caption,
            value,
            focused: true,
            enabled: true,
            edit,
        } => Some(field_caret(
            caption,
            edit.map_or(*value, |edit| edit.before_caret(value)),
        )),
        _ => None,
    }
}

/// One body row, drawn according to what it is.
fn render_row(
    f: &mut Frame<'_>,
    row: &Row<'_>,
    style: Style,
    width: u16,
    area: Rect,
    framed: Rect,
) {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let (focused, enabled) = (row.is_focused(), row.is_enabled());
    let look = FieldLook {
        focused,
        edit: row.edit(),
    };
    let line = match row {
        Row::Divider => {
            render_rule(f, style, area.y, framed);
            return;
        }
        Row::Text(text) => Line::from(hotkey_spans(text, style, accent)).centered(),
        Row::Label(text) => Line::from(hotkey_spans(text, style, accent)),
        Row::Plain(text) => Line::from(Span::styled((*text).to_owned(), style)),
        Row::Path(message) => path_line(message, style, width),
        Row::Input { value, history, .. } => input_line(value, look, *history, style, width),
        Row::Choice { caption, value, .. } | Row::Field { caption, value, .. } => {
            captioned_box(caption, value, look, enabled, style, accent, width)
        }
        Row::Radio {
            caption,
            options,
            selected,
            ..
        } => radio_line(caption, options, *selected, focused, style),
        Row::Checkbox { label, checked, .. } => {
            checkbox_line(label, *checked, focused, enabled, style)
        }
    };
    f.render_widget(Paragraph::new(line).style(style), area);
}

/// A [`Row::Path`] in `width` cells: the path gives way before the words do.
fn path_line(message: &PathMessage<'_>, style: Style, width: u16) -> Line<'static> {
    let text = fit_quoting_path(
        &message.before,
        &message.path,
        &message.after,
        Cells::columns(width),
    );
    Line::from(Span::styled(text, style))
}

/// A rule across the dialog, joined to the frame at both ends.
///
/// Double, like the frame it meets: a single rule ending on a double border
/// leaves two half-joined corners, which is what makes a dialog look drawn by
/// two different programs.
pub(super) fn render_rule(f: &mut Frame<'_>, style: Style, y: u16, framed: Rect) {
    let rule = format!(
        "╠{}╣",
        "═".repeat(usize::from(framed.width.saturating_sub(2)))
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(rule, style))).style(style),
        Rect {
            y,
            height: 1,
            ..framed
        },
    );
}
