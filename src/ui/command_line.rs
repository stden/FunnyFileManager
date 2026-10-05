//! Purpose: Renders the always-visible command line: the windowed typed text, its caret and the ghost suggestion.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::models::{App, CommandLine, Overlay};

use super::command_prompt::{command_prompt, prompt_spans, prompt_width, window_prompt};
use super::{screen_u16, Cells};

mod window;
use window::{paint_selection, window_command_row};

/// Separates the command line's path from the typed command.
const PROMPT_SUFFIX: &str = "> ";

/// Render the command line — always visible like in Far Manager.
///
/// Shows `path>` with a cursor even before the first character is typed. While
/// the prompt is open, the typed text carries that caret **where it actually
/// is**. A trailing glyph would be a lie now that the caret can sit mid-line,
/// and a caret the user cannot see is the same as no caret at all.
pub(super) fn render_command_line(f: &mut Frame<'_>, app: &App, area: Rect) {
    let state = match &app.overlay {
        Overlay::CommandLine(state) => Some(state),
        _ => None,
    };

    let prompt_text = command_prompt(app, state);
    let width = Cells::columns(area.width);

    let Some(state) = state else {
        render_inactive_prompt(f, area, prompt_text, width);
        return;
    };

    let (prompt, row) = window_command_row(state, prompt_text, width);
    let typed_from = prompt_width(&prompt);
    let caret = typed_from + Cells::of(&row.head);
    render_command_row(f, area, &prompt, row, width);
    paint_selection(f, area, &state.line, typed_from, caret);
}

fn render_inactive_prompt(
    f: &mut Frame<'_>,
    area: Rect,
    prompt_text: Vec<crate::prompt::Segment>,
    width: Cells,
) {
    // Prompt closed: keep one cell for the cursor. The command line is
    // always present in Far Manager, and hiding its cursor until the first
    // printable key makes that first input look as if it went nowhere.
    let caret = " ".to_owned();
    let prompt = window_prompt(prompt_text, width.saturating_sub(Cells::of(&caret)));
    render_command_row(
        f,
        area,
        &prompt,
        CommandRow {
            head: String::new(),
            caret,
            rest: String::new(),
            ghost: String::new(),
        },
        width,
    );
}

fn command_caret_text(state: &CommandLine) -> (String, &str, &str) {
    let after = state.line.after_cursor();
    // Ghost text is only ever offered with the caret at the end (see
    // `popup::command_line::refresh_ghost`), so it can never collide with
    // `after` — but read it defensively rather than assuming.
    let ghost = if state.line.at_end() {
        state.ghost.as_deref().unwrap_or("")
    } else {
        ""
    };

    // The caret shows the character it sits on; at the end of the line it
    // borrows the first ghost character, so the suggestion reads as one
    // continuous word instead of being pushed a cell to the right.
    let caret_char = after
        .chars()
        .next()
        .or_else(|| ghost.chars().next())
        .unwrap_or(' ');
    let caret: String = caret_char.into();
    // Whichever run the caret consumed a character from continues after it.
    let (rest, ghost_rest) = if after.is_empty() {
        ("", ghost.get(caret_char.len_utf8()..).unwrap_or(""))
    } else {
        (after.get(caret_char.len_utf8()..).unwrap_or(""), ghost)
    };

    (caret, rest, ghost_rest)
}

/// The four already-windowed runs that make up an open command line.
struct CommandRow {
    /// Typed text before the caret.
    head: String,
    /// The single displayed cell under the caret.
    caret: String,
    /// Typed text after the caret.
    rest: String,
    /// History suggestion beyond the caret.
    ghost: String,
}

/// Paint one command-line row from its pre-windowed runs.
fn render_command_row(
    f: &mut Frame<'_>,
    area: Rect,
    prompt: &[crate::prompt::Segment],
    row: CommandRow,
    width: Cells,
) {
    let caret_column = prompt_width(prompt) + Cells::of(&row.head);
    let cursor_position = command_cursor(area, caret_column, &row.caret);
    let used = prompt_width(prompt)
        + Cells::of(&row.head)
        + Cells::of(&row.caret)
        + Cells::of(&row.rest)
        + Cells::of(&row.ghost);
    let padding = " ".repeat(width.saturating_sub(used).get());

    let typed = Style::default().fg(Color::White).bg(Color::Black);
    let mut spans = prompt_spans(prompt);
    spans.extend([
        Span::styled(row.head, typed),
        // A reversed cell, not a "_" glyph: the caret has to be legible when it
        // sits *on* a character rather than after the last one.
        Span::styled(
            row.caret,
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Span::styled(row.rest, typed),
        Span::styled(
            row.ghost,
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(padding, Style::default().bg(Color::Black)),
    ]);

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black));
    f.render_widget(paragraph, area);
    if let Some(position) = cursor_position {
        // Ratatui hides the host cursor on frames that do not set a position.
        // Keep it on the painted cell; the yellow backing remains visible even
        // during the terminal cursor's blink-off phase.
        f.set_cursor_position(position);
    }
}

fn command_cursor(area: Rect, caret_column: Cells, caret: &str) -> Option<(u16, u16)> {
    if caret.is_empty() || area.width == 0 || area.height == 0 {
        None
    } else {
        let column = screen_u16(caret_column.get()).min(area.width.saturating_sub(1));
        Some((area.x.saturating_add(column), area.y))
    }
}
