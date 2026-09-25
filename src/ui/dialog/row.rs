//! Purpose: One line of a dialog's body — a sentence, a caption, a path, a
//! text field, a checkbox, a radio row, a drop-down or a divider — and the
//! columns each needs, so the frame is sized before it is drawn.

use super::super::Cells;
use super::HOTKEY_MARK;
use crate::status::PathMessage;

/// Columns an input field claims regardless of what is in it.
pub(super) const INPUT_WIDTH: usize = 34;

/// One line of a dialog's body.
pub(in crate::ui) enum Row<'a> {
    /// A sentence, centred — what a warning is made of.
    Text(&'a str),
    /// A field's caption, left-aligned above it.
    Label(&'a str),
    /// Text left-aligned exactly as given, with no hotkey: a note, where `&`
    /// is a character like any other.
    Plain(&'a str),
    /// A message quoting a path, drawn as [`Row::Plain`] is, except that a
    /// row too narrow for it cuts the path from its head and keeps at least
    /// its last component, as the panel's info row does. Cut at its end, the
    /// row would lose the very name it quotes the path for.
    Path(&'a PathMessage<'a>),
    /// A text field, sunk into the dialog on its own ground.
    ///
    /// `history` draws Far's `↓` marker at the right edge, which is how a
    /// field says it remembers what was typed into it before.
    Input {
        value: &'a str,
        focused: bool,
        history: bool,
    },
    /// A checkbox, drawn as Far draws it: `[x] Label`.
    ///
    /// `enabled` is drawn dimmed and skipped by the cursor, for an option that
    /// does not apply to this particular dialog.
    Checkbox {
        label: &'a str,
        checked: bool,
        focused: bool,
        enabled: bool,
    },
    /// A row of mutually exclusive choices: `Caption: (•) One ( ) Two`.
    ///
    /// All on one line, as Far lays them out. Three short options read faster
    /// side by side than stacked, and the dialog is already tall.
    Radio {
        caption: &'a str,
        options: &'a [&'a str],
        selected: usize,
        focused: bool,
    },
    /// A caption and a closed drop-down: `Caption: [value]↓`.
    Choice {
        caption: &'a str,
        value: &'a str,
        focused: bool,
    },
    /// A caption and a text field on the same line, drawn like a [`Row::Choice`]
    /// so that a group of the two lines its boxes up.
    ///
    /// `enabled` is dimmed and skipped by the cursor, for a field that does not
    /// apply to the answer currently given above it.
    Field {
        caption: &'a str,
        value: &'a str,
        focused: bool,
        enabled: bool,
    },
    /// A rule across the inner box, dividing one group of controls from the
    /// next.
    Divider,
}

impl<'a> Row<'a> {
    pub const fn input(value: &'a str, focused: bool, history: bool) -> Self {
        Self::Input {
            value,
            focused,
            history,
        }
    }

    pub const fn checkbox(label: &'a str, checked: bool, focused: bool) -> Self {
        Self::Checkbox {
            label,
            checked,
            focused,
            enabled: true,
        }
    }
}

/// The least room a captioned control gives its value, before the `↓`.
pub(super) const CHOICE_WIDTH: usize = 20;

/// Columns a caption claims before its value box begins.
///
/// A minimum rather than a fixed width: short captions are padded so the boxes
/// under one another start in the same column, which is how Far lays a group
/// out, and a caption longer than this simply takes the room it needs.
const CAPTION_WIDTH: usize = 12;

/// Blanks between a caption and its box — at least one, and enough to reach
/// [`CAPTION_WIDTH`].
pub(super) fn caption_gap(caption: &str) -> usize {
    let plain = Cells::of(&caption.replace(HOTKEY_MARK, "")).get();
    CAPTION_WIDTH.max(plain + 1) - plain
}

/// The marker Far puts at the right of a field that has a history or a list.
pub(super) const DROP_MARKER: char = '↓';

/// How one radio option is drawn when it is, and is not, the chosen one.
pub(super) const RADIO_ON: &str = "(•) ";
pub(super) const RADIO_OFF: &str = "( ) ";
/// What stands between two radio options, so `Default ( ) Copy` does not
/// read as `Default( ) Copy`.
pub(super) const RADIO_GAP: &str = " ";

impl Row<'_> {
    /// The columns this row needs, so the frame can be sized before it is drawn.
    pub(super) fn width(&self) -> Cells {
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
    pub(super) fn is_focused(&self) -> bool {
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

    pub(super) fn is_enabled(&self) -> bool {
        match self {
            Self::Field { enabled, .. } | Self::Checkbox { enabled, .. } => *enabled,
            _ => true,
        }
    }
}
