//! Purpose: The unlocked vault's tree view (F6): each group on its own row,
//! its matching entries indented under it, scrolled to keep the cursor's
//! entry on screen.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::super::palette;
use crate::kdbx::vault::VaultTreeRow;
use crate::kdbx::Vault;

pub(super) fn tree_content(vault: &Vault, rows: usize) -> Vec<Line<'static>> {
    let mut content = vec![Line::from(Span::styled(
        format!("Filter: {}_  Tree", vault.filter()),
        Style::default()
            .fg(palette::DIALOG_FG)
            .add_modifier(Modifier::BOLD),
    ))];
    append_visible_tree(&mut content, vault, rows);
    if let Some(notice) = &vault.notice {
        content.push(Line::from(notice.clone()));
    }
    content.push(Line::from(
        "Enter Copy | Ctrl+R Password | F4 Edit | F6 List | F8 Delete | Ctrl+Z Undo | Esc Close",
    ));
    content
}

fn append_visible_tree(content: &mut Vec<Line<'static>>, vault: &Vault, rows: usize) {
    vault.with_tree_layout(|layout| {
        let selected_pos = vault.selected();
        let selected_row = layout.match_row.get(selected_pos).copied().unwrap_or(0);
        let first = selected_row.saturating_sub(rows.saturating_sub(1));
        for row in layout
            .rows
            .get(first..)
            .unwrap_or_default()
            .iter()
            .take(rows)
        {
            if let Some(line) = format_tree_row(vault, *row, selected_pos) {
                content.push(line);
            }
        }
    });
}

fn format_tree_row(vault: &Vault, row: VaultTreeRow, selected_pos: usize) -> Option<Line<'static>> {
    match row {
        VaultTreeRow::Group { group_idx } => {
            let group = vault.groups().get(group_idx)?;
            let name = group.last().map_or("/", String::as_str);
            Some(Line::styled(
                format!("{}[{name}]", "  ".repeat(group.len())),
                Style::default().fg(palette::DRIVE_FG),
            ))
        }
        VaultTreeRow::Entry {
            group_idx,
            entry_idx,
            match_pos,
        } => format_entry_row(vault, group_idx, entry_idx, match_pos == selected_pos),
    }
}

fn format_entry_row(
    vault: &Vault,
    group_idx: usize,
    entry_idx: usize,
    active: bool,
) -> Option<Line<'static>> {
    let depth = vault.groups().get(group_idx)?.len();
    let entry = vault.entries().get(entry_idx)?;
    let marker = if active { "► " } else { "  " };
    let base = Style::default().fg(palette::DRIVE_FG);
    let style = if active {
        base.bg(palette::MENU_SELECTED_BG)
    } else {
        base
    };
    Some(Line::styled(
        format!(
            "{}{marker}{} ({})",
            "  ".repeat(depth + 1),
            entry.title,
            entry.username
        ),
        style,
    ))
}
