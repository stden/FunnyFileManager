//! Purpose: The unlocked vault's entry list: column widths budgeted in
//! cells, the filter and header rows, and one row per matching entry with
//! its password masked unless `Ctrl+R` revealed it on the cursor row.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::{fit_name, palette, Cells};
use super::tree::tree_content;
use crate::kdbx::Vault;
use crate::keepass::EntrySummary;

/// Cells the "► " / "  " cursor marker occupies.
pub(super) const MARKER_WIDTH: Cells = Cells::new(2);

/// Cells reserved for the password column.
pub(super) const PASSWORD_WIDTH: Cells = Cells::new(24);

/// One space between adjacent columns.
pub(super) const COLUMN_GAP: Cells = Cells::new(1);

/// What a masked password looks like.
///
/// A **fixed** number of bullets, deliberately unrelated to the real length: a
/// mask that grew with the password would publish how long every secret in the
/// vault is to anyone glancing at the screen.
pub(super) const MASK: &str = "••••••••";

/// Column widths for one rendering of the entry list, in terminal cells.
pub(super) struct Columns {
    pub(super) title: Cells,
    pub(super) username: Cells,
    pub(super) group: Cells,
}

impl Columns {
    /// Split `inner_w` between the four columns.
    ///
    /// The password column is fixed and taken out first: it is the one the user
    /// came for, and letting it shrink with the terminal would ellipsize a
    /// revealed secret before anything else gave way.
    pub(super) fn fit(inner_w: Cells) -> Self {
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

    pub(super) fn header(&self) -> String {
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
    pub(super) fn row(
        &self,
        entry: &EntrySummary,
        password: Option<&str>,
        selected: bool,
    ) -> String {
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
            fit_name(password.unwrap_or(MASK), PASSWORD_WIDTH),
        )
    }
}

pub(super) fn unlocked_content(
    vault: &Vault,
    rows: usize,
    columns: &Columns,
) -> Vec<Line<'static>> {
    if vault.is_tree() {
        return tree_content(vault, rows);
    }
    let mut content = vec![
        Line::from(Span::styled(
            format!("Filter: {}_", vault.filter()),
            Style::default()
                .fg(palette::HOTKEY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            columns.header(),
            Style::default().fg(palette::DRIVE_FG),
        )),
    ];

    append_entries(&mut content, vault, rows, columns);
    if let Some(notice) = &vault.notice {
        content.push(Line::from(notice.clone()));
    }
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Enter Copy | Ctrl+U User | Ctrl+R Password | F4 Edit | F6 Tree | F8 Delete | Ctrl+Z Undo",
        Style::default().fg(palette::HOTKEY),
    )));

    content
}

fn append_entries(content: &mut Vec<Line<'static>>, vault: &Vault, rows: usize, columns: &Columns) {
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
        content.push(Line::styled(
            columns.row(entry, password, selected),
            if selected {
                Style::default()
                    .fg(palette::DRIVE_FG)
                    .bg(palette::MENU_SELECTED_BG)
            } else {
                Style::default().fg(palette::DRIVE_FG)
            },
        ));
    }
}
