//! Purpose: How a dialog's controls are drawn: text fields, radio rows,
//! checkboxes and captioned boxes, the hotkey letter picked out of a label,
//! and the colours of the focused control.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::{palette, Cells};
use super::row::{
    caption_gap, FieldEdit, CHOICE_WIDTH, DROP_MARKER, INPUT_WIDTH, RADIO_GAP, RADIO_OFF, RADIO_ON,
};
use super::HOTKEY_MARK;

/// How a text field is drawn: which of its rows, whether it has the focus,
/// and where its caret and selection are.
#[derive(Clone, Copy)]
pub(super) struct FieldLook {
    pub(super) focused: bool,
    pub(super) edit: Option<FieldEdit>,
}

pub(super) fn input_line(
    value: &str,
    look: FieldLook,
    history: bool,
    style: Style,
    width: u16,
) -> Line<'static> {
    // The field spans the box rather than sitting in a fixed-width
    // well: Far's does, and a short well beside empty dialog reads as a
    // layout that failed rather than as a place to type.
    let field = Style::default().bg(palette::FIELD_BG).fg(palette::FIELD_FG);
    let marker = usize::from(history);
    let room = usize::from(width).max(INPUT_WIDTH).saturating_sub(marker);
    let mut spans = field_spans(value, room, field, look);
    if history {
        spans.push(Span::styled(DROP_MARKER.to_string(), style));
    }
    Line::from(spans)
}

pub(super) fn radio_line(
    caption: &str,
    options: &[&str],
    selected: usize,
    focused: bool,
    style: Style,
) -> Line<'static> {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let focus = focused_control_style(style);
    let mut spans = hotkey_spans(caption, style, accent);
    spans.push(Span::styled(" ", style));
    for (index, option) in options.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(RADIO_GAP, style));
        }
        let chosen = index == selected;
        // Only the chosen option carries the focus highlight: a row of
        // three reversed options would say the cursor is on all three.
        let base = if focused && chosen { focus } else { style };
        spans.push(Span::styled(
            if chosen { RADIO_ON } else { RADIO_OFF },
            base,
        ));
        spans.extend(hotkey_spans(
            option,
            base,
            accent.bg(base.bg.unwrap_or(palette::DIALOG_BG)),
        ));
    }
    Line::from(spans)
}

pub(super) fn checkbox_line(
    label: &str,
    checked: bool,
    focused: bool,
    enabled: bool,
    style: Style,
) -> Line<'static> {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let focus = focused_control_style(style);
    let base = if !enabled {
        style.fg(palette::DISABLED_FG)
    } else if focused {
        focus
    } else {
        style
    };
    let accent = if enabled {
        accent.bg(base.bg.unwrap_or(palette::DIALOG_BG))
    } else {
        base
    };
    let mark = if checked { 'x' } else { ' ' };
    let mut spans = vec![Span::styled(format!("[{mark}] "), base)];
    spans.extend(hotkey_spans(label, base, accent));
    Line::from(spans)
}

/// `value` padded to `room` columns on the field's ground, bold when focused,
/// and its selection, while it has the focus, drawn selected.
fn field_spans(value: &str, room: usize, field: Style, look: FieldLook) -> Vec<Span<'static>> {
    let ground = if look.focused {
        field.add_modifier(Modifier::BOLD)
    } else {
        field
    };
    let padded = format!("{value:<room$}");
    let selection = look
        .edit
        .and_then(|edit| edit.selection)
        .filter(|_| look.focused);
    let parts = selection.and_then(|(start, end)| {
        Some((
            padded.get(..start)?,
            padded.get(start..end)?,
            padded.get(end..)?,
        ))
    });
    let Some((before, selected, after)) = parts else {
        return vec![Span::styled(padded, ground)];
    };
    let chosen = Style::default()
        .bg(palette::FIELD_SELECTED_BG)
        .fg(palette::FIELD_SELECTED_FG);
    vec![
        Span::styled(before.to_owned(), ground),
        Span::styled(selected.to_owned(), chosen),
        Span::styled(after.to_owned(), ground),
    ]
}

/// A caption and a boxed value on one line, which is how both a drop-down and
/// a captioned text field are drawn.
///
/// One function because the difference between them is what typing does, not
/// what they look like — and two copies of the arithmetic would put the boxes
/// of one group in two different columns.
pub(super) fn captioned_box<'a>(
    caption: &'a str,
    value: &'a str,
    look: FieldLook,
    enabled: bool,
    style: Style,
    accent: Style,
    width: u16,
) -> Line<'a> {
    let base = if enabled {
        style
    } else {
        style.fg(palette::DISABLED_FG)
    };
    let gap = caption_gap(caption);
    let mut spans = hotkey_spans(caption, base, if enabled { accent } else { base });
    spans.push(Span::styled(" ".repeat(gap), style));

    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + gap;
    let room = usize::from(width)
        .saturating_sub(taken + 1)
        .max(CHOICE_WIDTH);
    let field = Style::default().bg(palette::FIELD_BG).fg(if enabled {
        palette::FIELD_FG
    } else {
        palette::DISABLED_FG
    });
    spans.extend(field_spans(value, room, field, look));
    spans.push(Span::styled(DROP_MARKER.to_string(), base));
    Line::from(spans)
}

/// Where the caret goes in a [`Row::Field`], measured the way it is drawn.
pub(in crate::ui) fn field_caret(caption: &str, value: &str) -> u16 {
    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + caption_gap(caption);
    u16::try_from(taken + Cells::of(value).get()).unwrap_or(u16::MAX)
}

/// Split a label on its hotkey marker, colouring the letter that follows.
pub(super) fn hotkey_spans<'a>(label: &str, base: Style, accent: Style) -> Vec<Span<'a>> {
    let Some((before, after)) = label.split_once(HOTKEY_MARK) else {
        return vec![Span::styled(label.to_owned(), base)];
    };
    let mut chars = after.chars();
    let Some(key) = chars.next() else {
        return vec![Span::styled(before.to_owned(), base)];
    };
    vec![
        Span::styled(before.to_owned(), base),
        Span::styled(key.to_string(), accent),
        Span::styled(chars.as_str().to_owned(), base),
    ]
}

pub(super) fn focused_control_style(ground: Style) -> Style {
    let background = if ground.bg == Some(palette::WARNING_BG) {
        palette::WARNING_FOCUS_BG
    } else {
        palette::DIALOG_FOCUS_BG
    };
    ground.bg(background).fg(palette::DIALOG_FG)
}
