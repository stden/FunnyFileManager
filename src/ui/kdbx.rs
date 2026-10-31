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

mod entry_dialogs;
mod form;
mod list;
mod locked;
mod tree;

use ratatui::{
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{far_block, palette, Cells};
use crate::kdbx::{KdbxSession, KdbxState, Vault};
use entry_dialogs::{render_delete, render_password};
use list::{unlocked_content, Columns};
use locked::render_locked;

/// Non-entry rows of the unlocked dialog: the filter line, the column header,
/// a blank spacer, and the key footer.
const CHROME_ROWS: usize = 5;

/// Draw the vault browser: the unlock prompt while locked, the entry list once
/// unlocked.
pub(super) fn render_kdbx(
    f: &mut Frame<'_>,
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
    crate::path_display::label(&full).into_owned()
}

fn render_unlocked(
    f: &mut Frame<'_>,
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
fn render_browser(f: &mut Frame<'_>, name: &str, vault: &Vault) {
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_unlocked;

#[cfg(test)]
mod tests_list_notice;
