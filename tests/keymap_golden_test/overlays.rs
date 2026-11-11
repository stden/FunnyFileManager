//! Purpose: the overlay contexts of the golden oracle — the viewer, the
//! editor, the command line and its two modes, the task list and its search
//! bar, the WSL manager, the Language servers, the PATH editor and the
//! vault — each as its resolver and the popup over it read a key today.
//! The vault reads its keys from the default keymap.

use std::sync::LazyLock;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::action::command_line::{self, completion, search, LineCommand, LineEdit};
use funny_file_manager::action::editor::{self, EditorCommand};
use funny_file_manager::action::tasks::{self, TaskCommand};
use funny_file_manager::action::vault::{self, VaultCommand};
use funny_file_manager::action::wsl::{self, WslCommand};
use funny_file_manager::action::{lsp_editor, path_env, viewer, Bindings};
use funny_file_manager::keymap::{tables, Keymap};
use funny_file_manager::models::ListMotion;

use super::outcome::{consumed, debug, Outcome};

/// A character typed without Ctrl or Alt.
fn typed(event: KeyEvent) -> Option<char> {
    match event.code {
        KeyCode::Char(ch)
            if !event
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(ch)
        }
        _ => None,
    }
}

pub fn viewer_overlay(event: KeyEvent) -> Outcome {
    viewer::resolve(&Keymap::default().viewer, event)
        .map_or(Outcome::Nothing, |command| consumed(&command))
}

/// The editor reads its keys from the keymap: the defaults, matched exactly.
pub fn editor_overlay(event: KeyEvent) -> Outcome {
    consumed(&editor::resolve(&Keymap::default().editor, event))
}

/// A character types itself; every other key is the editor's to ignore.
pub fn editor_family(event: KeyEvent) -> Outcome {
    typed(event).map_or_else(
        || consumed(&EditorCommand::Ignore),
        |ch| consumed(&EditorCommand::Insert(ch)),
    )
}

pub fn command_line(event: KeyEvent) -> Outcome {
    command_line::resolve(&Keymap::default().command_line, event)
        .map_or(Outcome::Nothing, |command| consumed(&command))
}

pub fn line_family(event: KeyEvent) -> Outcome {
    typed(event).map_or(Outcome::Nothing, |ch| {
        consumed(&LineCommand::Edit(LineEdit::Insert(ch)))
    })
}

pub fn line_search(event: KeyEvent) -> Outcome {
    search::resolve(&Keymap::default().command_line_search, event)
        .map_or(Outcome::Nothing, |command| consumed(&command))
}

pub fn line_search_family(event: KeyEvent) -> Outcome {
    typed(event).map_or(Outcome::Nothing, |ch| {
        consumed(&search::LineSearchCommand::Extend(ch))
    })
}

/// The list's own command, or what the command line under it does with
/// the key (`popup::command_line::keys::editing_key`).
pub fn line_completion(event: KeyEvent) -> Outcome {
    completion::resolve(&Keymap::default().command_line_completion, event).map_or_else(
        || Outcome::FallsThrough(command_line(event).to_string()),
        |command| consumed(&command),
    )
}

pub fn task_list(event: KeyEvent) -> Outcome {
    consumed(&tasks::resolve(event))
}

/// Ctrl with a character is ignored, Alt with one searches, a digit starts
/// a PID jump, and every other key is the list's to ignore.
pub fn task_list_family(event: KeyEvent) -> Outcome {
    let command = match event.code {
        KeyCode::Char(_) if event.modifiers.contains(KeyModifiers::CONTROL) => TaskCommand::Ignore,
        KeyCode::Char(ch) if event.modifiers.contains(KeyModifiers::ALT) => {
            TaskCommand::Search(Some(ch))
        }
        KeyCode::Char(ch) if ch.is_ascii_digit() => TaskCommand::PidJump(ch),
        _ => TaskCommand::Ignore,
    };
    consumed(&command)
}

/// The bar's one key, after the list moves that close the bar and move
/// the cursor (`popup::tasks::bars::search`); the bar's own editing is a
/// fixed protocol and is not a key of the table.
pub fn task_list_search(event: KeyEvent) -> Outcome {
    if let TaskCommand::Move(
        motion @ (ListMotion::Up | ListMotion::Down | ListMotion::PageUp | ListMotion::PageDown),
    ) = tasks::resolve(event)
    {
        return Outcome::FallsThrough(debug(&TaskCommand::Move(motion)));
    }
    tasks::search::resolve(event).map_or(Outcome::Nothing, |command| consumed(&command))
}

/// The keymap's defaults: the dialog reads `App.keymap.wsl_manager`.
pub fn wsl_manager(event: KeyEvent) -> Outcome {
    static KEYS: LazyLock<Bindings<WslCommand>> = LazyLock::new(|| Keymap::default().wsl_manager);
    consumed(&wsl::resolve(&KEYS, event))
}

pub fn wsl_family(_: KeyEvent) -> Outcome {
    consumed(&WslCommand::Ignore)
}

/// The keymap's defaults, which the dialog reads since its switch: a key
/// nothing is bound to is swallowed.
pub fn language_servers(event: KeyEvent) -> Outcome {
    let keys = Keymap::default().language_servers;
    lsp_editor::resolve(&keys, event).map_or(Outcome::Nothing, |command| consumed(&command))
}

/// The keymap's default keys: every key is the list's, or swallowed.
pub fn path_editor(event: KeyEvent) -> Outcome {
    path_env::resolve(&Keymap::default().path_editor, event)
        .map_or(Outcome::Nothing, |command| consumed(&command))
}

/// The vault under the default keys of its table, which are the default
/// keymap's `vault` (`Keymap::default`).
pub fn vault_browser(event: KeyEvent) -> Outcome {
    vault::resolve(&tables::VAULT.defaults(), event)
        .map_or(Outcome::Nothing, |command| consumed(&command))
}

/// A character is typed into the password or the filter.
pub fn vault_family(event: KeyEvent) -> Outcome {
    typed(event).map_or(Outcome::Nothing, |ch| consumed(&VaultCommand::Type(ch)))
}
