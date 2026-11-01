//! Purpose: Find file (`Alt+F7`): the mask and containing-text dialog.

use crate::models::{App, Overlay};

/// Alt+F7: open the Find file dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::FindFile(Box::default()));
}

/// Run the typed mask and containing text from the open dialog.
pub fn accept(app: &mut App) {
    let Some((mask, containing)) = query(app) else {
        return;
    };
    crate::ops::menu::find_files(app, &mask, &containing);
}

fn query(app: &App) -> Option<(String, String)> {
    let Overlay::FindFile(dialog) = &app.overlay else {
        return None;
    };
    let mask = dialog.mask.text().trim();
    let mask = if mask.is_empty() {
        "*".to_owned()
    } else {
        mask.to_owned()
    };
    Some((mask, dialog.containing.text().trim().to_owned()))
}

#[cfg(test)]
mod tests;
