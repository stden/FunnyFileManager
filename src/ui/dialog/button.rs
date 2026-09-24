//! Purpose: A dialog's buttons: what each one is (default, focused,
//! enabled), the width it costs, the centred row they are drawn on, and the
//! columns each covers so a click lands on the button the user sees.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::{palette, Cells};
use super::controls::{focused_control_style, hotkey_spans};
use super::placement::Placement;
use super::HOTKEY_MARK;

/// Blank columns between two buttons.
pub(super) const BUTTON_GAP: &str = " ";

/// One button in a dialog.
pub(in crate::ui) struct Button<'a> {
    /// May carry a [`HOTKEY_MARK`], which is stripped before drawing.
    pub label: &'a str,
    /// Drawn `{ like this }` rather than `[ like this ]`. Far marks the button
    /// Enter starts on, so the user can see what a reflex will do.
    pub is_default: bool,
    /// Carries the cursor: highlighted, and what Enter activates now.
    pub is_focused: bool,
    /// Drawn dimmed and skipped by the cursor. Far shows a button that does
    /// not apply rather than hiding it, so the dialog keeps its shape and the
    /// user learns where the control will be when it does apply.
    pub is_enabled: bool,
}

impl<'a> Button<'a> {
    /// A plain button: present, active, not the default.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            is_default: false,
            is_focused: false,
            is_enabled: true,
        }
    }

    pub const fn default_button(mut self) -> Self {
        self.is_default = true;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }
}

impl Button<'_> {
    /// The width this button costs on the row, measured where it is drawn so
    /// the sizing and the drawing cannot disagree.
    pub(super) fn width(&self) -> Cells {
        // Two brackets and two blanks, plus the label without its marker.
        Cells::of(&self.label.replace(HOTKEY_MARK, "")) + Cells::new(4)
    }
}

/// The screen columns each button covers, in the order they were given.
///
/// The same centring `button_line` draws with, so a click lands on the button
/// the user is looking at.
pub(in crate::ui) fn button_spans(
    placement: &Placement,
    buttons: &[Button<'_>],
) -> Vec<(usize, u16, u16)> {
    let total = buttons
        .iter()
        .map(|button| button.width().get() + Cells::of(BUTTON_GAP).get())
        .sum::<usize>()
        .saturating_sub(Cells::of(BUTTON_GAP).get());

    let width = usize::from(placement.body.width);
    let mut left = placement.body.x + u16::try_from(width.saturating_sub(total) / 2).unwrap_or(0);
    let mut spans = Vec::with_capacity(buttons.len());
    for (index, button) in buttons.iter().enumerate() {
        let cells = u16::try_from(button.width().get()).unwrap_or(0);
        spans.push((index, left, left.saturating_add(cells)));
        left = left
            .saturating_add(cells)
            .saturating_add(u16::try_from(Cells::of(BUTTON_GAP).get()).unwrap_or(0));
    }
    spans
}

/// The buttons, centred, each with its hotkey letter picked out.
pub(super) fn button_line<'a>(buttons: &[Button<'_>], ground: Style) -> Line<'a> {
    let mut spans = Vec::new();
    for button in buttons {
        // The focused button is the one Enter acts on, so it is the one lit up.
        let base = button_style(button, ground);
        let (open, close) = if button.is_default {
            ("{ ", " }")
        } else {
            ("[ ", " ]")
        };
        spans.push(Span::styled(open, base));
        // A disabled button's hotkey is not picked out: the accent means "this
        // letter does something", and here it does not.
        spans.extend(hotkey_spans(
            button.label,
            base,
            if button.is_enabled {
                base.fg(palette::HOTKEY).add_modifier(Modifier::BOLD)
            } else {
                base
            },
        ));
        spans.push(Span::styled(close, base));
        spans.push(Span::styled(BUTTON_GAP, ground));
    }
    spans.pop();
    Line::from(spans).centered()
}

fn button_style(button: &Button<'_>, ground: Style) -> Style {
    if !button.is_enabled {
        ground.fg(palette::DISABLED_FG)
    } else if button.is_focused {
        focused_control_style(ground)
    } else {
        ground
    }
}
