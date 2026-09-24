//! Purpose: Drawing the generic input dialog and the yes/no question.
//!
//! Both are Far's plain boxes: a caption over a field and OK / Cancel, or a
//! sentence over Yes / No. What they are for is in the purpose's title, so
//! a feature that asks through them is named on screen without owning a
//! renderer.

use ratatui::Frame;

use super::{dialog, Cells};
use crate::models::{InputPrompt, Question};

/// The one-line input, its caret where the editor's cursor is.
pub(super) fn render_input(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    prompt: &InputPrompt,
) {
    let on_ok = !prompt.on_cancel();
    let title = prompt.purpose().title();
    let caret = dialog::render_tracked(
        f,
        tracker,
        &title,
        dialog::Ground::Prompt,
        &[
            dialog::Row::Label(prompt.purpose().label()),
            dialog::Row::input(prompt.input().text(), on_ok, false),
        ],
        &[
            dialog::Button::new("&OK").default_button().focused(on_ok),
            dialog::Button::new("&Cancel").focused(!on_ok),
        ],
    );
    if let Some((end, y)) = caret {
        // The box reports the end of the text; the caret sits wherever the
        // editor's cursor is, which may be mid-line after Left or Home.
        let after = u16::try_from(Cells::of(prompt.input().after_cursor()).get()).unwrap_or(0);
        f.set_cursor_position((end.saturating_sub(after), y));
    }
}

/// The question, drawn the way the quit confirmation is.
pub(super) fn render_question(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    question: &Question,
) {
    let text = question.purpose().text();
    let yes = question.answer().is_yes();
    dialog::render_tracked(
        f,
        tracker,
        question.purpose().title(),
        dialog::Ground::Prompt,
        &[dialog::Row::Text(&text)],
        &[
            dialog::Button::new("&Yes").default_button().focused(yes),
            dialog::Button::new("&No").focused(!yes),
        ],
    );
}
