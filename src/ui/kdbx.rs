//! Purpose: rendering for the `.kdbx` vault-browser overlay — the unlock prompt
//! and the entry list.
//!
//! # What may reach the screen
//!
//! The master password cannot: [`UnlockPrompt`](crate::kdbx::UnlockPrompt) has
//! no accessor for the plaintext at all, only [`masked`]-style bullet counts, so
//! there is nothing here to print by mistake. An entry password is drawn only
//! when the vault says it is revealed, which only `Ctrl+R` can arrange.
//!
//! [`masked`]: crate::kdbx::UnlockPrompt::masked
//!
//! # Widths
//!
//! Every column goes through [`fit_name`] / [`head_cells`], which budget in
//! terminal **cells**. A CJK title is one char and two columns; measuring in
//! chars would push the password column off the row.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, fit_name, head_cells, Cells, FAR_BLUE};
use crate::kdbx::{KdbxSession, KdbxState, Vault};
use crate::keepass::EntrySummary;

/// Cells the "► " / "  " cursor marker occupies.
const MARKER_WIDTH: Cells = Cells::new(2);

/// Cells reserved for the password column.
const PASSWORD_WIDTH: Cells = Cells::new(24);

/// One space between adjacent columns.
const COLUMN_GAP: Cells = Cells::new(1);

/// Non-entry rows of the unlocked dialog: the filter line, the column header,
/// a blank spacer, and the key footer.
const CHROME_ROWS: usize = 4;

/// What a masked password looks like.
///
/// A **fixed** number of bullets, deliberately unrelated to the real length: a
/// mask that grew with the password would publish how long every secret in the
/// vault is to anyone glancing at the screen.
const MASK: &str = "••••••••";

/// Draw the vault browser: the unlock prompt while locked, the entry list once
/// unlocked.
pub(super) fn render_kdbx(f: &mut Frame, session: &KdbxSession) {
    let name = session
        .path
        .file_name()
        .unwrap_or_else(|| session.path.as_os_str())
        .to_string_lossy()
        .into_owned();

    match &session.state {
        KdbxState::Locked(prompt) => render_locked(f, &name, prompt),
        KdbxState::Unlocked(vault) => render_unlocked(f, &name, vault),
    }
}

fn render_locked(f: &mut Frame, name: &str, prompt: &crate::kdbx::UnlockPrompt) {
    let area = centered_rect(60, 30, f.area());

    let mut content = vec![
        Line::from(Span::raw("Master password:")),
        // `masked()` is the only view of the field there is — one bullet per
        // typed character, never the characters.
        Line::from(Span::styled(
            format!("{}_", prompt.masked()),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        )),
        Line::from(""),
    ];
    if let Some(keyfile) = prompt.keyfile() {
        content.push(Line::from(Span::raw(format!(
            "Key file: {}",
            keyfile.display()
        ))));
    }
    if let Some(error) = prompt.error() {
        // Fixed, credential-free text — `kdbx::unlock_error_message` maps every
        // failure onto a phrase chosen here rather than forwarding the crate's.
        content.push(Line::from(Span::styled(
            error.to_owned(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Enter Unlock  ·  Esc Cancel",
        Style::default().fg(Color::Cyan),
    )));

    let block = Block::default()
        .title(format!(" KeePass: {name} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(FAR_BLUE));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::Cyan)),
        area,
    );
}

fn render_unlocked(f: &mut Frame, name: &str, vault: &Vault) {
    let area = centered_rect(85, 80, f.area());
    let inner_w = Cells::columns(area.width.saturating_sub(2));
    let inner_h = usize::from(area.height.saturating_sub(2));
    let rows = inner_h.saturating_sub(CHROME_ROWS).max(1);
    let columns = Columns::fit(inner_w);

    let mut content = vec![
        Line::from(Span::styled(
            format!("Filter: {}_", vault.filter()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            columns.header(),
            Style::default().fg(Color::White),
        )),
    ];

    // Scroll only as far as it takes to keep the cursor on screen, so the list
    // stays put while the user pages through the top of it.
    let first = vault.selected().saturating_sub(rows.saturating_sub(1));
    let visible = vault
        .matches()
        .iter()
        .skip(first)
        .take(rows)
        .filter_map(|&index| vault.entries().get(index));
    for (offset, entry) in visible.enumerate() {
        let position = first + offset;
        let selected = position == vault.selected();
        // A revealed password belongs to the entry that was selected when
        // `Ctrl+R` was pressed, and the vault drops it whenever the cursor or
        // the filter moves — so it can only ever be drawn on the cursor row.
        let password = if selected { vault.revealed() } else { None };
        content.push(Line::from(Span::styled(
            columns.row(entry, password, selected),
            if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Cyan)
            },
        )));
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Enter Copy password  ·  Ctrl+U Copy user  ·  Ctrl+R Reveal  ·  Esc Lock",
        Style::default().fg(Color::Cyan),
    )));

    let block = Block::default()
        .title(format!(
            " KeePass: {name} — {} of {} entries ",
            vault.matches().len(),
            vault.entries().len()
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(FAR_BLUE));

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::Cyan)),
        area,
    );
}

/// Column widths for one rendering of the entry list, in terminal cells.
struct Columns {
    title: Cells,
    username: Cells,
    group: Cells,
}

impl Columns {
    /// Split `inner_w` between the four columns.
    ///
    /// The password column is fixed and taken out first: it is the one the user
    /// came for, and letting it shrink with the terminal would ellipsize a
    /// revealed secret before anything else gave way.
    fn fit(inner_w: Cells) -> Self {
        // Sharing a budget out is ratio arithmetic, not width arithmetic, so
        // it happens on the raw count and every result becomes a width again.
        let body = inner_w
            .saturating_sub(MARKER_WIDTH + PASSWORD_WIDTH + COLUMN_GAP + COLUMN_GAP + COLUMN_GAP)
            .get()
            .max(3);
        let title = body * 2 / 5;
        let username = body * 2 / 5;
        Self {
            title: Cells::new(title),
            username: Cells::new(username),
            group: Cells::new(body - title - username),
        }
    }

    fn header(&self) -> String {
        let gap = " ".repeat(COLUMN_GAP.get());
        format!(
            "{}{}{gap}{}{gap}{}{gap}{}",
            " ".repeat(MARKER_WIDTH.get()),
            fit_name("Title", self.title),
            fit_name("User", self.username),
            fit_name("Group", self.group),
            fit_name("Password", PASSWORD_WIDTH),
        )
    }

    /// One entry row. `password` is `Some` only for a revealed entry; every
    /// other row shows [`MASK`].
    fn row(&self, entry: &EntrySummary, password: Option<&str>, selected: bool) -> String {
        let gap = " ".repeat(COLUMN_GAP.get());
        // Joined for display only. `KeePass` allows '/' inside a group name, so
        // this string must never be parsed back into a path — `EntrySummary`
        // keeps the structural `group_path` for that.
        let group = if entry.group_path.is_empty() {
            "/".to_owned()
        } else {
            entry.group_path.join("/")
        };
        format!(
            "{}{}{gap}{}{gap}{}{gap}{}",
            if selected { "► " } else { "  " },
            fit_name(&entry.title, self.title),
            fit_name(&entry.username, self.username),
            fit_name(&group, self.group),
            head_cells(password.unwrap_or(MASK), PASSWORD_WIDTH),
        )
    }
}

#[cfg(test)]
mod tests;
