//! Purpose: open the prompts, menus and confirmations the panels view asks through.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{self, App, CommandLine, NamePrompt, Overlay};
use crate::popup::process_popup_input;
use crate::{copy, menu, ops};

pub(super) fn toggle_help(app: &mut App) {
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help);
    }
}

pub(super) fn open_drive_menu(app: &mut App, side: models::PanelSide) {
    app.set_overlay(Overlay::DriveSelection {
        side,
        selected_index: 0,
        rows: crate::models::drive_rows(),
    });
}

pub(super) fn open_rename_prompt(app: &mut App) {
    let name = app
        .get_active_panel()
        .get_selected_entry()
        .map(|entry| entry.name.clone())
        .filter(|name| name != "..");
    match name {
        Some(name) => app.set_overlay(Overlay::NamePrompt(NamePrompt::rename(
            crate::models::FileName::new(name),
        ))),
        None => app.set_status(crate::status::Status::CommandOutput(
            "Nothing to rename".to_owned(),
        )),
    }
}

/// Open the command line already holding `ch`.
///
/// The character is *replayed through the overlay's own key handler* rather
/// than pushed into the buffer directly, so the first keystroke of a command
/// gets everything a later one does — the history ghost in particular. Two
/// insertion paths would be one too many, and the second would be the one
/// nobody remembers to keep in step.
pub(super) fn open_command_line_with(app: &mut App, ch: char) {
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    process_popup_input(app, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()));
}

/// F5 and F6: ask where the files are going, offering the other panel.
///
/// Far asks rather than acting, and the reason is not caution for its own
/// sake: the destination is the one thing a transfer cannot be undone about,
/// and before the dialog the only way to say "somewhere else" was to go and
/// point the other panel at it first. F6 gets the same dialog because moving
/// within a directory *is* renaming, and Far treats them as one question.
pub(super) fn open_transfer_prompt(app: &mut models::App, purpose: copy::Purpose) {
    let sources = ops::get_source_paths(app);
    let what = match sources.as_slice() {
        [] => return,
        [one] => format!(
            "\"{}\"",
            one.file_name().unwrap_or_default().to_string_lossy()
        ),
        many => format!("{} files", many.len()),
    };
    let destination = app.get_inactive_panel().path.clone();
    app.set_overlay(models::Overlay::CopyPrompt(Box::new(
        copy::CopyPrompt::for_purpose(purpose, what, &destination),
    )));
}

/// Gray + / Gray −, opening on the extension under the cursor.
pub(super) fn open_mask_prompt(app: &mut App, purpose: models::MaskPurpose) {
    let selected = app
        .get_active_panel()
        .get_selected_entry()
        .map(|entry| entry.name.clone());
    let configured = app.settings.get(menu::SettingsPage::Masks);
    let suggestion = if configured.is_empty() {
        models::MaskPrompt::suggest(selected.as_deref())
    } else {
        configured.to_owned()
    };
    app.set_overlay(Overlay::MaskPrompt(models::MaskPrompt::new(
        purpose, suggestion,
    )));
}

/// F9: open the menu bar on the heading for the active panel.
pub(super) fn open_menu(app: &mut App) {
    let heading = match app.active_panel {
        models::PanelSide::Left => menu::Heading::Left,
        models::PanelSide::Right => menu::Heading::Right,
    };
    app.set_menu(menu::Menu::open(heading));
}
