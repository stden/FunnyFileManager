//! Purpose: rendering for the `.kdbx` overlay — unlock, tree/list browsing,
//! password, deletion, and entry editing dialogs.
//!
//! # What may reach the screen
//!
//! The master password is drawn in clear only while the user has explicitly
//! toggled the `◉` control. An entry password is drawn only in the explicit
//! password or edit dialog.
//!
//! [`masked`]: crate::kdbx::UnlockPrompt::masked
//!
//! # Widths
//!
//! Every column goes through [`fit_name`], which budgets in
//! terminal **cells**. A CJK title is one char and two columns; measuring in
//! chars would push the password column off the row.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{dialog, far_block, fit_name, palette, Cells};
use crate::kdbx::{KdbxSession, KdbxState, Vault};
use crate::keepass::EntrySummary;
mod form;

/// Cells the "► " / "  " cursor marker occupies.
const MARKER_WIDTH: Cells = Cells::new(2);

/// Cells reserved for the password column.
const PASSWORD_WIDTH: Cells = Cells::new(24);

/// One space between adjacent columns.
const COLUMN_GAP: Cells = Cells::new(1);

/// Non-entry rows of the unlocked dialog: the filter line, the column header,
/// a blank spacer, and the key footer.
const CHROME_ROWS: usize = 5;

/// What a masked password looks like.
///
/// A **fixed** number of bullets, deliberately unrelated to the real length: a
/// mask that grew with the password would publish how long every secret in the
/// vault is to anyone glancing at the screen.
const MASK: &str = "••••••••";

/// Draw the vault browser: the unlock prompt while locked, the entry list once
/// unlocked.
pub(super) fn render_kdbx(
    f: &mut Frame,
    session: &KdbxSession,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let path = display_path(&session.path);

    match &session.state {
        KdbxState::Locked(prompt) => render_locked(f, &path, prompt, tracker),
        KdbxState::Unlocked(vault) => {
            if let Some(form) = vault.form() {
                tracker.clear();
                form::render(f, &path, form);
            } else {
                let mode = if session.access == crate::kdbx::Access::Edit {
                    "Edit"
                } else {
                    "View"
                };
                render_unlocked(f, &format!("{path} [{mode}]"), vault, tracker);
            }
        }
    }
}

fn display_path(path: &std::path::Path) -> String {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |current| current.join(path))
    };
    full.display().to_string()
}

fn render_locked(
    f: &mut Frame,
    name: &str,
    prompt: &crate::kdbx::UnlockPrompt,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let password = clipped_password(f.area(), name, prompt, tracker.offset);
    let detail = locked_details(prompt);
    let mut rows = vec![
        dialog::Row::Label("Master password:"),
        dialog::Row::input(&password, true, false),
    ];
    rows.extend(detail.iter().map(|text| dialog::Row::Text(text)));
    let buttons = [
        dialog::Button::new("Unlock").default_button(),
        dialog::Button::new("◉"),
        dialog::Button::new("Cancel"),
    ];
    if let Some(position) = dialog::render_tracked(
        f,
        tracker,
        &format!("KeePass: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    ) {
        f.set_cursor_position(position);
    }
}

fn clipped_password(
    screen: ratatui::layout::Rect,
    name: &str,
    prompt: &crate::kdbx::UnlockPrompt,
    offset: (i32, i32),
) -> String {
    let rows = [
        dialog::Row::Label("Master password:"),
        dialog::Row::input("", true, false),
    ];
    let buttons = [
        dialog::Button::new("Unlock").default_button(),
        dialog::Button::new("◉"),
        dialog::Button::new("Cancel"),
    ];
    let width = dialog::place(screen, &format!("KeePass: {name}"), &rows, &buttons, offset)
        .map_or(0, |placement| {
            usize::from(placement.body.width.saturating_sub(1))
        });
    prompt.visible_text().chars().take(width).collect()
}

fn render_unlocked(
    f: &mut Frame,
    name: &str,
    vault: &Vault,
    tracker: &mut crate::mouse::DialogMouse,
) {
    use crate::kdbx::vault::Page;
    match vault.page {
        Page::Password(_) => {
            render_password(f, name, vault, tracker);
        }
        Page::DeleteConfirm(_) => {
            render_delete(f, name, vault, tracker);
        }
        Page::Browse(_) => {
            tracker.clear();
            render_browser(f, name, vault);
        }
    }
}

/// The unlocked vault is a working view, not a dialog: it takes the whole
/// terminal like the F4 editor, so a long entry list is not squeezed into 80 %
/// of the screen. The locked prompt below stays a centred dialog — it is a
/// question about the file under the cursor, and the panels stay visible
/// behind it.
fn render_browser(f: &mut Frame, name: &str, vault: &Vault) {
    let area = f.area();
    let inner_w = Cells::columns(area.width.saturating_sub(2));
    let inner_h = usize::from(area.height.saturating_sub(2));
    let rows = inner_h.saturating_sub(CHROME_ROWS).max(1);
    let columns = Columns::fit(inner_w);

    let content = unlocked_content(vault, rows, &columns);

    let block = far_block()
        .title(
            Line::from(format!(
                " KeePass: {name} — {} of {} entries ",
                vault.matches().len(),
                vault.entries().len()
            ))
            .centered(),
        )
        .border_style(Style::default().fg(palette::DRIVE_FG))
        .style(Style::default().bg(palette::MENU_BG));

    // No drop shadow: it falls outside a full-screen area, and nothing shows
    // through behind it.
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(palette::DRIVE_FG)),
        area,
    );
}

fn render_password(
    f: &mut Frame,
    name: &str,
    vault: &Vault,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let entry = vault.selected_entry();
    let title = entry.map_or("", |entry| entry.title.as_str());
    let username = entry.map_or("", |entry| entry.username.as_str());
    let password = vault.revealed().unwrap_or_default();
    let rows = [
        dialog::Row::Label("Entry:"),
        dialog::Row::Text(title),
        dialog::Row::Label("User:"),
        dialog::Row::Text(username),
        dialog::Row::Label("Password:"),
        dialog::Row::input(password, false, false),
        dialog::Row::Text("Enter Copy  ·  Esc Close"),
    ];
    let buttons = [
        dialog::Button::new("Copy").default_button(),
        dialog::Button::new("Close"),
    ];
    dialog::render_tracked(
        f,
        tracker,
        &format!("KeePass password: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
    );
}

fn render_delete(
    f: &mut Frame,
    name: &str,
    vault: &Vault,
    tracker: &mut crate::mouse::DialogMouse,
) {
    let title = vault
        .selected_entry()
        .map_or("", |entry| entry.title.as_str());
    let rows = [
        dialog::Row::Text("Delete this KeePass entry?"),
        dialog::Row::Label(title),
        dialog::Row::Text("The database will be backed up before writing."),
    ];
    let buttons = [
        dialog::Button::new("Delete").default_button(),
        dialog::Button::new("Cancel"),
    ];
    dialog::render_tracked(
        f,
        tracker,
        &format!("Delete from: {name}"),
        dialog::Ground::Prompt,
        &rows,
        &buttons,
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
            fit_name(password.unwrap_or(MASK), PASSWORD_WIDTH),
        )
    }
}

#[cfg(test)]
mod tests;

fn locked_details(prompt: &crate::kdbx::UnlockPrompt) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(keyfile) = prompt.keyfile() {
        details.push(format!("Key file: {}", keyfile.display()));
    }
    if let Some(error) = prompt.error() {
        details.push(error.to_owned());
    }
    details.push("Enter Unlock  ·  Esc Cancel".to_owned());
    details
}

fn unlocked_content(vault: &Vault, rows: usize, columns: &Columns) -> Vec<Line<'static>> {
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

fn tree_content(vault: &Vault, rows: usize) -> Vec<Line<'static>> {
    let tree = build_tree(vault);
    let selected_row = tree.iter().position(|(_, selected)| *selected).unwrap_or(0);
    let first = selected_row.saturating_sub(rows.saturating_sub(1));
    let mut content = vec![Line::from(Span::styled(
        format!("Filter: {}_  Tree", vault.filter()),
        Style::default()
            .fg(palette::HOTKEY)
            .add_modifier(Modifier::BOLD),
    ))];
    append_visible_tree(&mut content, tree, first, rows);
    if let Some(notice) = &vault.notice {
        content.push(Line::from(notice.clone()));
    }
    content.push(Line::from(
        "Enter Copy | Ctrl+R Password | F4 Edit | F6 List | F8 Delete | Ctrl+Z Undo | Esc Close",
    ));
    content
}

fn build_tree(vault: &Vault) -> Vec<(String, bool)> {
    let selected = vault.matches().get(vault.selected()).copied();
    let mut tree = Vec::new();
    for group in vault.groups() {
        let name = group.last().map_or("/", String::as_str);
        tree.push((format!("{}[{name}]", "  ".repeat(group.len())), false));
        append_group_entries(&mut tree, vault, group, selected);
    }
    tree
}

fn append_group_entries(
    tree: &mut Vec<(String, bool)>,
    vault: &Vault,
    group: &[String],
    selected: Option<usize>,
) {
    for &index in vault.matches() {
        let Some(entry) = vault.entries().get(index) else {
            continue;
        };
        if entry.group_path == group {
            let active = Some(index) == selected;
            let marker = if active { "► " } else { "  " };
            tree.push((
                format!(
                    "{}{marker}{} ({})",
                    "  ".repeat(group.len() + 1),
                    entry.title,
                    entry.username
                ),
                active,
            ));
        }
    }
}

fn append_visible_tree(
    content: &mut Vec<Line<'static>>,
    tree: Vec<(String, bool)>,
    first: usize,
    rows: usize,
) {
    content.extend(
        tree.into_iter()
            .skip(first)
            .take(rows)
            .map(|(text, selected)| {
                let style = if selected {
                    Style::default()
                        .fg(palette::DRIVE_FG)
                        .bg(palette::MENU_SELECTED_BG)
                } else {
                    Style::default().fg(palette::DRIVE_FG)
                };
                Line::styled(text, style)
            }),
    );
}
