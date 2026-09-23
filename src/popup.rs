//! Handles modal overlay input and routes each dialog to its owner.

use crate::models::{App, Overlay};
use crate::search::{SearchField, SearchPrompt};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod disk_cleanup;
mod menu_dialog;
mod path_env;
pub(crate) use editor::paste_text as paste_editor_text;

/// Handle keyboard input for popup overlays (help, viewer, editor, dialogs).
///
/// Overlays are modal windows that appear on top of the panels.
/// While an overlay is open, keys go here instead of the main handler.
/// Returns true if the key was consumed by an overlay.
pub fn process_popup_input(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    let modifiers = key.modifiers;
    match &app.overlay {
        Overlay::None => false,
        Overlay::MenuDialog(_) => menu_dialog::process(app, code, modifiers),
        Overlay::Wsl(_) => wsl::process(app, code, modifiers),
        Overlay::PathEditor(_) => path_env::process(app, code, modifiers),
        Overlay::Viewer(_) => viewer::process(app, code, modifiers),
        Overlay::Editor(_) => process_editor_input(app, code, modifiers),
        Overlay::Help => confirmation::help(app, code),
        Overlay::Information(_) => confirmation::information(app, code),
        Overlay::QuitConfirm(_) => confirmation::quit(app, code),
        Overlay::NamePrompt(_) => name_prompt::process(app, code, modifiers),
        Overlay::MaskPrompt(_) => mask_prompt::process(app, code, modifiers),
        Overlay::CopyPrompt(_) => copy_prompt::process(app, code, modifiers),
        Overlay::Menu(_) => menu::process(app, code, modifiers),
        Overlay::CommandLine(_) => command_line::process(app, key),
        Overlay::DriveSelection { .. } => drives::process(app, code),
        Overlay::DeleteConfirm(_) => confirmation::delete(app, code),
        Overlay::NewFileInput(_) => new_file::process(app, code, modifiers),
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(_) => kdbx::process_kdbx_input(app, code, modifiers),
        Overlay::DiskCleanup(_) => disk_cleanup::process(app, code, modifiers),
        _ => process_menu_dialog(app, code, modifiers),
    }
}

/// The Files-menu dialogs the completed F9 menu brought, one owner each.
///
/// Every owner receives the modifiers, whether it reads them yet or not: the
/// History list wants `Ctrl+Enter` and the view history `Shift+Enter`, and an
/// owner may not edit this router to ask for them later.
///
/// A second and third function only to keep each under the line limit; the
/// split follows the menu headings.
fn process_menu_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match &app.overlay {
        Overlay::WipeConfirm(_) => wipe::process(app, code, modifiers),
        Overlay::ArchivePrompt(_) => archive_prompt::process(app, code, modifiers),
        Overlay::ExtractPrompt(_) => extract_prompt::process(app, code, modifiers),
        Overlay::ArchiveCommands(_) => archive_commands::process(app, code, modifiers),
        Overlay::Attributes(_) => attributes::process(app, code, modifiers),
        Overlay::ApplyCommand(_) => apply::process(app, code, modifiers),
        Overlay::Describe(_) => describe::process(app, code, modifiers),
        _ => process_commands_dialog(app, code, modifiers),
    }
}

/// The Commands- and Options-menu dialogs. Every variant is named in one of
/// the three routers; the fallback here is only for the arms above.
fn process_commands_dialog(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match &app.overlay {
        Overlay::FindFile(_) => findfile::process(app, code, modifiers),
        Overlay::CommandHistory(_) => history_list::process(app, code, modifiers),
        Overlay::FindFolder(_) => findfolder::process(app, code, modifiers),
        Overlay::ViewHistory(_) => view_history::process(app, code, modifiers),
        Overlay::FolderHistory(_) => folder_history::process(app, code, modifiers),
        Overlay::UserMenu(_) => usermenu::process(app, code, modifiers),
        Overlay::Associations(_) => associations::process(app, code, modifiers),
        Overlay::FolderShortcuts(_) => shortcuts::process(app, code, modifiers),
        Overlay::Filter(_) => filter::process(app, code, modifiers),
        Overlay::Screens(_) => screens::process(app, code, modifiers),
        Overlay::Tasks(_) => tasks::process(app, code, modifiers),
        Overlay::Hotplug(_) => hotplug::process(app, code, modifiers),
        Overlay::SortModes(_) => sortmenu::process(app, code, modifiers),
        Overlay::Settings(_) => settings::process(app, code, modifiers),
        Overlay::SessionEdit(_) => session_edit::process(app, code, modifiers),
        Overlay::PluginCommands(_) => plugin_commands::process(app, code, modifiers),
        Overlay::InputPrompt(_) => input_prompt::process(app, code, modifiers),
        Overlay::Question(_) => question::process(app, code, modifiers),
        _ => false,
    }
}

/// Lines jumped by PgUp/PgDn in the built-in editor.
const EDITOR_PAGE: usize = 20;

/// Keys belonging to the F7 dialog, which owns the keyboard while it is open.
///
/// Tab and the arrows walk the controls, Space flips whichever checkbox the
/// cursor is on, and Enter runs the search from wherever the cursor is — so a
/// user who only ever types a word and presses Enter never meets the rest.
fn process_search_prompt(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };
    let Some(prompt) = editor.search.prompt.as_mut() else {
        return false;
    };
    // Crossterm reports Ctrl+A as `Char('a')` with CONTROL: a chord is a
    // command, never text, as in every other prompt.
    let typed = !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match code {
        KeyCode::Esc | KeyCode::F(10) => editor.search.prompt = None,
        KeyCode::Tab | KeyCode::Down => prompt.field = prompt.field.next(),
        KeyCode::BackTab | KeyCode::Up => prompt.field = prompt.field.previous(),
        KeyCode::Char(' ') if prompt.field.is_toggle() => prompt.toggle(),
        KeyCode::Backspace if prompt.field == SearchField::Query => {
            prompt.query.pop();
        }
        KeyCode::Char(ch) if typed && prompt.field == SearchField::Query => prompt.query.push(ch),
        KeyCode::Enter => submit_search(app),
        _ => {}
    }
    true
}

fn submit_search(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let Some(mut prompt) = editor.search.prompt.take() else {
        return;
    };
    if prompt.field == SearchField::Cancel {
        return;
    }
    if let Some(direction) = prompt.field.direction() {
        prompt.direction = direction;
    }
    if let Some(search) = prompt.to_search() {
        run_search(app, search);
    }
}

/// Shift+F7: run the last search again from where the caret is now.
fn repeat_search(app: &mut App) {
    let Some(search) = app
        .get_editor()
        .and_then(|editor| editor.search.last.clone())
    else {
        // Nothing to repeat, so Shift+F7 asks what to look for — which is what
        // the user meant, and saves a trip through F7.
        if let Some(editor) = app.get_editor_mut() {
            editor.search.prompt = Some(SearchPrompt::default());
        }
        return;
    };
    run_search(app, search);
}

/// Move the caret to the next match, or record that there was none.
fn run_search(app: &mut App, search: crate::search::Search) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let found = editor.find(&search);
    editor.search.not_found = (!found).then(|| search.needle.as_str().to_owned());
    // Remembered either way: repeating a search that failed is how the user
    // checks after editing the file.
    editor.search.last = Some(search);
}

mod apply;
mod archive_commands;
mod archive_prompt;
mod associations;
mod attributes;
mod command_line;
mod confirmation;
mod copy_prompt;
mod describe;
mod drives;
mod editor;
mod extract_prompt;
mod filter;
mod findfile;
mod findfolder;
mod folder_history;
mod history_list;
mod hotplug;
mod input_prompt;
mod mask_prompt;
mod menu;
mod name_prompt;
mod netbox;
mod new_file;
mod plugin_commands;
mod question;
pub(crate) mod quickview;
pub(crate) mod remote;
mod screens;
mod session_edit;
pub(crate) mod sessions;
mod settings;
mod shortcuts;
mod sortmenu;
mod stub;
mod tasks;
pub(crate) mod tree;
mod usermenu;
mod view_history;
mod viewer;
mod wipe;
mod wsl;
use editor::process_editor_input;
#[cfg(feature = "keepass")]
mod kdbx;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_command_line_completion;
#[cfg(test)]
mod tests_command_line_editing;
#[cfg(test)]
mod tests_console_and_editor_modes;
#[cfg(test)]
mod tests_dialog_keys;
#[cfg(test)]
mod tests_editor_search;
#[cfg(test)]
mod tests_viewer_keys;
