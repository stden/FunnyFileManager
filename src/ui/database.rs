//! Purpose: Draw the database explorer: its tree, or the columns or rows of one
//! table, with a status line and a key hint. Full screen, on the panel's blue.
//! Everything shown comes from the explorer's state; nothing is read here.

use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use super::{palette, FAR_BLUE};
use crate::models::database::{DatabaseExplorer, View};

mod columns;
mod rows;
mod tree;

const HINT: &str = "Right/Left open · Enter rows · Tab columns · PgUp/PgDn pages · F5 reload · Esc back · F10 close";

pub(super) fn render(f: &mut Frame<'_>, explorer: &DatabaseExplorer) {
    let screen = f.area();
    f.render_widget(Clear, screen);
    f.render_widget(Block::default().style(ground()), screen);
    let [body, status, hint] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(screen);
    let lines = match explorer.view() {
        View::Tree => tree::lines(explorer),
        View::Columns(columns) => columns::lines(explorer, columns.path, columns.scroll),
        View::Rows(rows) => rows::lines(rows),
    };
    f.render_widget(Paragraph::new(lines).style(ground()), body);
    f.render_widget(
        Paragraph::new(status_line(explorer)).style(ground()),
        status,
    );
    f.render_widget(Paragraph::new(HINT).style(ground().reversed()), hint);
}

pub(super) fn ground() -> Style {
    Style::default().fg(palette::CODE_NORMAL).bg(FAR_BLUE)
}

/// Loading while a request is pending; otherwise the notice, when there is one.
fn status_line(explorer: &DatabaseExplorer) -> Line<'static> {
    let text = if explorer.is_loading() {
        "Loading…".to_owned()
    } else {
        explorer.notice().unwrap_or("").to_owned()
    };
    Line::from(text)
}

#[cfg(test)]
mod tests;
