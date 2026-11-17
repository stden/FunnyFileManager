//! Purpose: Draws the Wipe dialog (Alt+Del) — F8's red question with another
//! verb. The names are listed because a name is what the user checks before
//! Enter; the ground is red because the colour lands before the text is read.
//!
//! The list rule is `dialog::preview_lines`, shared with the Delete dialog and
//! every other red question that names what it acts on.

use std::path::{Path, PathBuf};

use ratatui::Frame;

use super::dialog;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    files: &[PathBuf],
    noun: &str,
) {
    let mut message = vec![question(files.len(), noun)];
    dialog::preview_lines(files, |path| shown_name(path), &mut message);

    dialog::render_warning(
        f,
        tracker,
        "Wipe",
        &message,
        &[
            dialog::Button::new("Wipe").default_button().focused(true),
            dialog::Button::new("Cancel"),
        ],
    );
}

/// The one entry by its kind — a file, a folder or a link — or the count.
fn question(count: usize, noun: &str) -> String {
    match count {
        1 => format!("Do you wish to wipe the {noun}?"),
        count => format!("Do you wish to wipe {count} items?"),
    }
}

/// `..` has no file name; the whole path is still better than a blank row.
fn shown_name(path: &Path) -> String {
    super::entry_label(path)
}

#[cfg(test)]
mod tests;
