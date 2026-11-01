//! Purpose: The unlocked vault's tree view (F6): each group on its own row,
//! its matching entries indented under it, scrolled to keep the cursor's
//! entry on screen.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::palette;
use crate::kdbx::Vault;

pub(super) fn tree_content(vault: &Vault, rows: usize) -> Vec<Line<'static>> {
    let tree = build_tree(vault);
    let selected_row = tree.iter().position(|(_, selected)| *selected).unwrap_or(0);
    let first = selected_row.saturating_sub(rows.saturating_sub(1));
    let mut content = vec![Line::from(Span::styled(
        format!("Filter: {}_  Tree", vault.filter()),
        Style::default()
            .fg(palette::DIALOG_FG)
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
