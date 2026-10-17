//! Purpose: The placeholder every not-yet-built dialog draws — Far's frame
//! with the row's title, and one line saying the feature is on its way.
//!
//! A frame rather than nothing: a menu row that opens a window says it was
//! heard, and a row that visibly does nothing reads as a dead key. The line
//! is honest about what is behind the frame. When the last feature has its
//! own renderer, this file goes.

use ratatui::Frame;

use super::dialog;

/// What the frame says while the feature is on its way.
const COMING: &str = "Coming in this release";

pub(super) fn render_coming(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    title: &str,
) {
    dialog::render_tracked(
        f,
        tracker,
        title,
        dialog::Ground::Prompt,
        &[dialog::Row::Text(COMING)],
        &[dialog::Button::new("&Ok").default_button().focused(true)],
    );
}
