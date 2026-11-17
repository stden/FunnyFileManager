//! Purpose: How many columns a dialog row needs, and whether its control has
//! the cursor and is enabled.

use super::{caption_gap, Cells, Row, CHOICE_WIDTH, HOTKEY_MARK, INPUT_WIDTH, RADIO_GAP, RADIO_ON};

impl Row<'_> {
    /// The columns this row needs, so the frame can be sized before it is drawn.
    pub(in crate::ui::dialog) fn width(&self) -> Cells {
        let plain = |text: &str| Cells::of(&text.replace(HOTKEY_MARK, ""));
        match self {
            Self::Text(text) | Self::Label(text) => plain(text),
            Self::Plain(text) => Cells::of(text),
            Self::Path(message) => {
                Cells::of(&message.before) + Cells::of(&message.path) + Cells::of(&message.after)
            }
            // The field is given room to grow rather than sized to what has
            // been typed, or the dialog would resize on every keystroke.
            Self::Input { value, .. } => Cells::of(value).max(Cells::new(INPUT_WIDTH)),
            Self::Checkbox { label, .. } => plain(label) + Cells::new(4),
            Self::Radio {
                caption, options, ..
            } => {
                let choices: Cells = options
                    .iter()
                    .map(|option| plain(option) + Cells::of(RADIO_ON))
                    .sum();
                let gaps = Cells::of(&RADIO_GAP.repeat(options.len().saturating_sub(1)));
                plain(caption) + Cells::new(1) + choices + gaps
            }
            Self::Choice { caption, .. } | Self::Field { caption, .. } => {
                plain(caption) + Cells::new(caption_gap(caption) + CHOICE_WIDTH + 1)
            }
            Self::Divider => Cells::ZERO,
        }
    }
}

impl Row<'_> {
    pub(in crate::ui::dialog) fn is_focused(&self) -> bool {
        match self {
            Self::Input { focused, .. }
            | Self::Radio { focused, .. }
            | Self::Choice { focused, .. }
            | Self::Field { focused, .. }
            | Self::Checkbox { focused, .. } => *focused,
            Self::Text(_) | Self::Label(_) | Self::Plain(_) | Self::Path(_) | Self::Divider => {
                false
            }
        }
    }

    pub(in crate::ui::dialog) fn is_enabled(&self) -> bool {
        match self {
            Self::Field { enabled, .. } | Self::Checkbox { enabled, .. } => *enabled,
            _ => true,
        }
    }
}
