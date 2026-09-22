//! Purpose: Find file (`Alt+F7`): open the dialog; the walk arrives with the feature.

use crate::menu::MenuCommand;
use crate::models::{App, Overlay};

/// Alt+F7: open the Find file dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::FindFile(Box::default()));
}

/// Run the typed mask from the open dialog.
pub fn accept(app: &mut App) {
    let Some(mask) = mask_from_dialog(app) else {
        return;
    };
    let _ = crate::ops::menu::submit(app, MenuCommand::FindFile, &mask);
}

fn mask_from_dialog(app: &App) -> Option<String> {
    let Overlay::FindFile(dialog) = &app.overlay else {
        return None;
    };
    let mask = dialog.mask.trim();
    Some(if mask.is_empty() {
        "*".to_owned()
    } else {
        mask.to_owned()
    })
}

#[cfg(test)]
mod tests;
