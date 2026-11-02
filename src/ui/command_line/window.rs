//! Purpose: The command line's visible window and the selection painted over it.

use ratatui::{layout::Rect, style::Style, Frame};

use super::super::command_prompt::{prompt_width, window_prompt};
use super::super::{palette, screen_u16, tail_cells, take_cells, Cells};
use super::{command_caret_text, CommandRow, PROMPT_SUFFIX};
use crate::models::CommandLine;

/// Draw the line's selection over the row: it ends at the caret or starts
/// there, and it is cut to the typed text the row shows.
pub(super) fn paint_selection(
    f: &mut Frame<'_>,
    area: Rect,
    line: &crate::lineedit::LineEditor,
    typed_from: Cells,
    caret: Cells,
) {
    let Some(selected) = line.selected_text() else {
        return;
    };
    let width = Cells::of(selected).get();
    let caret = caret.get();
    let ends_at_caret = line
        .selection()
        .is_some_and(|range| range.end == line.cursor());
    let (start, end) = if ends_at_caret {
        (caret.saturating_sub(width).max(typed_from.get()), caret)
    } else {
        (caret, caret.saturating_add(width))
    };
    let cells = Rect::new(
        area.x.saturating_add(screen_u16(start)),
        area.y,
        screen_u16(end.saturating_sub(start)),
        1,
    )
    .intersection(area);
    f.buffer_mut().set_style(
        cells,
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG),
    );
}

pub(super) fn window_command_row(
    state: &CommandLine,
    prompt_text: Vec<crate::prompt::Segment>,
    width: Cells,
) -> (Vec<crate::prompt::Segment>, CommandRow) {
    let before = state.line.before_cursor();
    let (caret, rest, ghost_rest) = command_caret_text(state);
    // The typed text is windowed FIRST, against everything but the prompt and
    // the caret. It is the one part of this row the user cannot read anywhere
    // else. The pre-caret text keeps its *tail* so the caret stays on screen
    // however long the line grows — a window anchored at the start would let
    // the user type blind past the terminal width.
    let budget = width.saturating_sub(Cells::of(&caret) + Cells::of(PROMPT_SUFFIX));
    let head = tail_cells(before, budget);

    // The path is next in line and is the part that can grow without bound, so
    // it is what gets cut before the tail or the ghost is even considered.
    let reserved = Cells::of(&head) + Cells::of(&caret) + Cells::of(rest) + Cells::of(ghost_rest);
    let prompt = window_prompt(prompt_text, width.saturating_sub(reserved));

    // Clamp what follows the caret to the room actually left: the path may not
    // have shrunk by the full amount requested. The tail yields before the
    // ghost does, and the typed text before the caret never yields at all.
    let used = prompt_width(&prompt) + Cells::of(&head) + Cells::of(&caret);
    let (rest_shown, rest_cells) = take_cells(rest, width.saturating_sub(used));
    let (ghost_shown, _) = take_cells(ghost_rest, width.saturating_sub(used + rest_cells));

    (
        prompt,
        CommandRow {
            head,
            caret,
            rest: rest_shown,
            ghost: ghost_shown,
        },
    )
}

#[cfg(test)]
mod tests;
