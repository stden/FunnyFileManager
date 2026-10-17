//! Purpose: The hotkey letter in a label, the caret column of a field, and the
//! colour of the control that has the cursor.

use ratatui::{style::Style, text::Span};

use super::{caption_gap, palette, Cells, HOTKEY_MARK};

/// Where the caret goes in a [`Row::Field`], measured the way it is drawn.
pub(in crate::ui) fn field_caret(caption: &str, value: &str) -> u16 {
    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + caption_gap(caption);
    u16::try_from(taken + Cells::of(value).get()).unwrap_or(u16::MAX)
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
