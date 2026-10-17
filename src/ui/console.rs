//! Purpose: The Ctrl+O console screen, where a session's command output lives.
//!
//! Far Manager keeps command output on the console screen and draws the panels
//! over it. FFM runs on ratatui's alternate screen, which has no such screen,
//! so the transcript lives in [`crate::console::ConsoleLog`] and is painted
//! here — only for `Ctrl+O`'s console view. The panel view does not paint it
//! at all: a ratatui widget writes symbols only where its text reaches, so a
//! layer "under" the panel chrome shows through every gap in it.
//!
//! Split out of `ui.rs` because the console is one coherent surface: its ground,
//! its text, its prompt placement and its scrollback note only make sense
//! together, and `ui.rs` is the layout of everything else.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use super::{head_cells, render_command_line, render_completion_menu, Cells};
use crate::console::ConsoleLine;
use crate::models::{App, Overlay};
use crate::scroll::ScrollBack;

/// Draw what `Ctrl+O` reveals: a console screen, as close to a real one as a
/// TUI gets.
///
/// The prompt sits on the row **directly after the newest transcript line**,
/// which is the whole shape of a terminal: output grows downwards and the
/// cursor follows it. Pinning the prompt to the screen bottom instead left a
/// band of dead black between the last line of output and the place the next
/// command is typed — the one thing no console ever shows. A transcript
/// long enough to fill the screen pushes the prompt onto the last row anyway,
/// which is what scrolling looks like.
///
/// The prompt is drawn whether or not it is open, so the console always has a
/// cursor waiting on it; typing is what opens it.
pub(super) fn render_view(f: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    // Every row but the one the prompt needs.
    let rows = render_text(
        f,
        app,
        Rect {
            height: area.height - 1,
            ..area
        },
    );
    let row = Rect {
        y: area.y + rows,
        height: 1,
        ..area
    };
    // An open prompt always gets the row, whatever the scroll says: dispatch
    // snaps back to the bottom when the prompt opens, and a prompt hidden
    // behind a note would be a prompt typed into blind.
    if app.console.scroll() == ScrollBack::BOTTOM || matches!(app.overlay, Overlay::CommandLine(_))
    {
        render_command_line(f, app, row);
        render_completion_menu(f, app, row);
    } else {
        // Scrolled back, the prompt has scrolled off the bottom exactly as it
        // does in a terminal — so the row it vacated carries how far
        // down the newest line is. That is the one thing a terminal's scrollbar
        // tells the reader and this console otherwise could not.
        render_scroll_note(f, app, row);
    }
}

/// Paint the black ground the whole frame is drawn onto.
///
/// Separate from the text because the two cover different rectangles in Ctrl+O
/// mode, and because the ground is needed even for an empty log — a
/// transparent one would let the terminal's own colours show through the gaps.
pub(super) fn render_backdrop(f: &mut Frame<'_>, area: Rect) {
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );
}

/// Draw the console transcript into `area`, oldest visible line at the top, and
/// report how many rows it filled.
///
/// One row per line, filling downwards from the top like the console screen
/// this stands in for. Command echoes are bright so a long transcript reads as
/// a sequence of commands rather than an undifferentiated wall; stderr is red.
///
/// The row count is what lets the prompt follow the output instead of being
/// pinned to the screen bottom, so it is returned from the one place that knows
/// it rather than recomputed against a second copy of the same clamping.
pub(super) fn render_text(f: &mut Frame<'_>, app: &App, area: Rect) -> u16 {
    let height = usize::from(area.height);
    let width = Cells::columns(area.width);
    let lines: Vec<Line<'_>> = app
        .console
        .visible(height)
        .into_iter()
        .map(|line| {
            let style = match line {
                ConsoleLine::Command(_) => Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                ConsoleLine::Output(_) => Style::default().fg(Color::Gray).bg(Color::Black),
                ConsoleLine::Error(_) => Style::default().fg(Color::LightRed).bg(Color::Black),
            };
            // Cut rather than wrap: one transcript line per row keeps the
            // scroll arithmetic in `ConsoleLog` honest — a wrapped line would
            // occupy rows the log does not know it spent.
            Line::from(Span::styled(head_cells(line.text(), width), style))
        })
        .collect();

    // Before the count, because `lines` is moved into the widget.
    let rows = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .min(area.height);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Black)),
        area,
    );
    rows
}

/// The one row a real console cannot supply: where the reader is in the
/// scrollback.
///
/// Only drawn while scrolled away from the newest line, taking the row the
/// prompt has just scrolled off. A terminal answers this with a scrollbar; with
/// no scrollbar to draw, the count is the honest substitute — and at the
/// bottom, where there is nothing to say, the console says nothing and looks
/// like a console.
fn render_scroll_note(f: &mut Frame<'_>, app: &App, row: Rect) {
    let note = format!("↓ {} more line(s) below", app.console.scroll().lines());
    f.render_widget(Clear, row);
    f.render_widget(
        Paragraph::new(head_cells(&note, Cells::columns(row.width)))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        row,
    );
}
