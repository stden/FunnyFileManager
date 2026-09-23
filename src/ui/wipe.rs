//! Purpose: Draws the Wipe dialog (Alt+Del) — F8's red question with another
//! verb. The names are listed because a name is what the user checks before
//! Enter; the ground is red because the colour lands before the text is read.
//!
//! The list rule is the one `render_delete_confirm` in `overlays` keeps; two
//! copies for now, a shared renderer when a third question of this shape
//! arrives.

use std::path::{Path, PathBuf};

use ratatui::Frame;

use super::dialog;

/// How many names are listed before the rest become a count.
const PREVIEW: usize = 5;

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    files: &[PathBuf],
) {
    let mut message = vec![question(files.len())];
    message.extend(files.iter().take(PREVIEW).map(|path| shown_name(path)));
    if files.len() > PREVIEW {
        message.push(format!("… and {} more", files.len() - PREVIEW));
    }

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

fn question(count: usize) -> String {
    match count {
        1 => "Do you wish to wipe the file?".to_owned(),
        count => format!("Do you wish to wipe {count} items?"),
    }
}

/// `..` has no file name; the whole path is still better than a blank row.
fn shown_name(path: &Path) -> String {
    super::entry_label(path)
}

#[cfg(test)]
mod tests;
