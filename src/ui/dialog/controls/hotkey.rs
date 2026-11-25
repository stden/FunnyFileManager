//! Purpose: The hotkey letter in a label, the caret column and windowing of a
//! field, and the colour of the control that has the cursor.

use ratatui::{style::Style, text::Span};

use super::super::super::{head_cells, tail_cells, take_cells};
use super::super::row::{caption_gap, Row, CHOICE_WIDTH, INPUT_WIDTH};
use super::{palette, Cells, FieldLook, HOTKEY_MARK};

/// Where the caret goes in a [`Row::Field`], measured the way it is drawn.
pub(in crate::ui) fn field_caret(caption: &str, value: &str) -> u16 {
    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + caption_gap(caption);
    u16::try_from(taken + Cells::of(value).get()).unwrap_or(u16::MAX)
}

pub(in crate::ui::dialog) fn input_room(width: u16, history: bool) -> usize {
    usize::from(width)
        .max(INPUT_WIDTH)
        .saturating_sub(usize::from(history))
}

pub(in crate::ui::dialog) fn captioned_room(caption: &str, width: u16) -> (usize, usize) {
    let gap = caption_gap(caption);
    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + gap;
    let room = usize::from(width)
        .saturating_sub(taken + 1)
        .max(CHOICE_WIDTH);
    (taken, room)
}

/// The visible `(before_caret, from_caret)` slices of `value` inside a box of
/// `room` cells.
pub(in crate::ui::dialog) fn window_field(
    value: &str,
    room: usize,
    look: FieldLook,
) -> (String, String) {
    let room_cells = Cells::new(room);
    if !look.focused {
        return (head_cells(value, room_cells), String::new());
    }
    let before = look.edit.map_or(value, |edit| edit.before_caret(value));
    let after = look
        .edit
        .and_then(|edit| value.get(edit.caret..))
        .unwrap_or("");
    if Cells::of(value) <= room_cells && (look.edit.is_none() || Cells::of(before) < room_cells) {
        return (before.to_owned(), after.to_owned());
    }
    let head = tail_cells(before, Cells::new(room.saturating_sub(1)));
    let (tail, _) = take_cells(after, room_cells.saturating_sub(Cells::of(&head)));
    (head, tail)
}

pub(in crate::ui::dialog) fn row_caret(row: &Row<'_>, width: u16) -> Option<u16> {
    let col = match row {
        Row::Input {
            value,
            history,
            focused: true,
            edit,
        } => field_caret_offset(value, input_room(width, *history), *edit),
        Row::Field {
            caption,
            value,
            focused: true,
            enabled: true,
            edit,
        } => {
            let (taken, room) = captioned_room(caption, width);
            taken + field_caret_offset(value, room, *edit)
        }
        _ => return None,
    };
    u16::try_from(col)
        .ok()
        .map(|c| c.min(width.saturating_sub(1)))
}

fn field_caret_offset(
    value: &str,
    room: usize,
    edit: Option<super::super::row::FieldEdit>,
) -> usize {
    let (head, _) = window_field(
        value,
        room,
        FieldLook {
            focused: true,
            edit,
        },
    );
    Cells::of(&head).get().min(room)
}

/// Split a label on its hotkey marker, colouring the letter that follows.
pub(in crate::ui::dialog) fn hotkey_spans<'a>(
    label: &str,
    base: Style,
    accent: Style,
) -> Vec<Span<'a>> {
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

pub(in crate::ui::dialog) fn focused_control_style(ground: Style) -> Style {
    let background = if ground.bg == Some(palette::WARNING_BG) {
        palette::WARNING_FOCUS_BG
    } else {
        palette::DIALOG_FOCUS_BG
    };
    ground.bg(background).fg(palette::DIALOG_FG)
}
