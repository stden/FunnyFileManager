//! Purpose: Far's modal dialogs — the frame, the fields, the buttons, the shadow.
//!
//! One renderer for every modal, parameterised by the colour it stands on. Far
//! uses that colour to say what kind of question is being asked: red for
//! anything that can lose data, grey for anything that merely wants an answer.
//! The colour lands before the text is read, which for the destructive case is
//! exactly when a reflexive Enter would otherwise already have been pressed.
//!
//! The shape is Far's too: a double frame carrying the title, an inner
//! single-line box around the body, and `├───┤` rules dividing that box into
//! groups — the last of which is always the buttons.
//!
//! Sized to its content and centred, because a fixed-percentage box either
//! wraps a short question across a huge empty field or clips a long file name.

mod body;
mod button;
mod controls;
mod placement;
mod row;
mod shadow;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{far_block, palette};
use body::{render_body, render_rule};
pub(super) use button::Button;
use button::{button_line, button_spans};
pub(super) use controls::field_caret;
use placement::frame_area;
pub(super) use placement::place;
pub(super) use row::Row;
pub(super) use shadow::render_shadow;

/// The character that marks the next letter as a control's hotkey.
///
/// Far's own convention. It is needed because the hotkey is not always the
/// first letter — `Fu&zzy`, `Find pre&vious` — so picking one positionally
/// would be wrong exactly where the user most needs it to be right.
const HOTKEY_MARK: char = '&';

/// What kind of question a dialog is asking, which is what its colour says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ground {
    /// Something can be lost by answering wrongly.
    Warning,
    /// Something merely needs to be supplied.
    Prompt,
}

impl Ground {
    fn style(self) -> Style {
        match self {
            Self::Warning => Style::default()
                .bg(palette::WARNING_BG)
                .fg(palette::WARNING_FG),
            Self::Prompt => Style::default()
                .bg(palette::DIALOG_BG)
                .fg(palette::DIALOG_FG),
        }
    }
}

/// Draw a dialog and record where its parts landed, for the next click.
///
/// The recording is the point: a dialog is sized to its own content, so unlike
/// the panels its geometry cannot be recomputed from the screen size alone.
/// The loop draws immediately before it reads an event, so what is recorded
/// here is exactly the frame the user was looking at.
pub(super) fn render_tracked(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    title: &str,
    ground: Ground,
    rows: &[Row<'_>],
    buttons: &[Button<'_>],
) -> Option<(u16, u16)> {
    let offset = tracker.offset;
    let caret = render_at(f, title, ground, rows, buttons, offset);
    if let Some(placement) = place(f.area(), title, rows, buttons, offset) {
        tracker.area = Some(placement.area);
        tracker.body = Some(placement.body);
        tracker.buttons_y = placement.buttons_y;
        tracker.buttons = button_spans(&placement, buttons);
        // Nothing is drawn over a dialog yet; one that hangs a popup over
        // itself records it after this.
        tracker.covered = None;
    } else {
        tracker.clear();
    }
    caret
}

/// The same dialog, shifted by however far it has been dragged.
pub(super) fn render_at(
    f: &mut Frame<'_>,
    title: &str,
    ground: Ground,
    rows: &[Row<'_>],
    buttons: &[Button<'_>],
    offset: (i32, i32),
) -> Option<(u16, u16)> {
    let screen = f.area();
    let placement = place(screen, title, rows, buttons, offset)?;
    let area = placement.area;

    render_shadow(f, area, screen);

    let style = ground.style();
    // The surface first, then the frame inset into it.
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(style), area);

    let framed = frame_area(area);
    let frame = far_block()
        .title(Line::from(Span::styled(format!(" {title} "), style)).centered())
        .border_style(style)
        .style(style);
    let body = placement.body;
    f.render_widget(frame, framed);

    let caret = render_body(f, rows, style, body, framed);

    render_rule(f, style, body.y + body.height.saturating_sub(2), framed);
    f.render_widget(
        Paragraph::new(button_line(buttons, style)).style(style),
        Rect {
            y: body.y + body.height - 1,
            height: 1,
            ..body
        },
    );
    caret
}

/// Names a dialog lists before it starts counting instead: enough to check
/// what an action is about, few enough to leave room for the buttons.
const PREVIEWED_NAMES: usize = 5;

/// Appends the `name` of the first few `items` to `into`, one per line, then
/// `… and N more` for the items left unnamed.
///
/// Far names what a dialog is about and counts the rest: the name is what the
/// user checks before pressing Enter, and a bare count is the one thing that
/// cannot be checked. Only the listed items are named, so a dialog over a
/// hundred thousand files formats five names, not all of them. The count
/// comes from the same slice as the names, so the two cannot disagree.
pub(super) fn preview_lines<T>(
    items: &[T],
    name: impl FnMut(&T) -> String,
    into: &mut Vec<String>,
) {
    into.extend(items.iter().take(PREVIEWED_NAMES).map(name));
    let unnamed = items.len().saturating_sub(PREVIEWED_NAMES);
    if unnamed > 0 {
        into.push(format!("… and {unnamed} more"));
    }
}

/// A red warning dialog: a few sentences and some buttons.
pub(super) fn render_warning(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    title: &str,
    message: &[String],
    buttons: &[Button<'_>],
) {
    let rows: Vec<Row<'_>> = message.iter().map(|line| Row::Text(line)).collect();
    render_tracked(f, tracker, title, Ground::Warning, &rows, buttons);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_preview;
#[cfg(test)]
mod tests_radio;
