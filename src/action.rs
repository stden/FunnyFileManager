//! Input → Action DSL.
//!
//! This module separates *what key was pressed* from *what the app does*. The
//! event loop in [`crate`] resolves a raw [`KeyEvent`] into a typed [`Action`]
//! via [`resolve`], then a dispatcher applies it. Keeping the keymap in one
//! pure, exhaustively-testable function (no terminal, no I/O) mirrors the
//! approach [`crate::command`] already uses for the command line, and makes the
//! full set of main-view bindings reviewable at a glance.
//!
//! Main-view bindings resolve here; [`editor`] declares the built-in editor's
//! commands using the same binding syntax. Modal routing remains owned by
//! [`crate::popup::process_popup_input`].

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::menu::MenuCommand;
use crate::models::{PanelSide, PanelView};

#[macro_use]
mod keymap;
mod menu;

pub mod editor;
pub mod wsl;

mod commands;
mod function_keys;
mod scope;

pub use commands::{
    DialogCommand, FileCommand, Navigation, PanelCommand, SelectionCommand, ToolCommand,
};
use function_keys::resolve_function_key;
pub use scope::{menu_scope, ActionScope};

/// A single user-triggerable operation in the main dual-panel view.
///
/// Every variant corresponds to exactly one effect applied to the
/// [`App`](crate::models::App); the mapping from keys to actions is [`resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MenuCommand(MenuCommand),
    /// Select the active panel's file presentation mode.
    PanelView(PanelView),
    /// Typed dialog intent, dispatched by its own owner.
    Dialog(DialogCommand),
    /// Typed selection intent, dispatched by its own owner.
    Selection(SelectionCommand),
    /// Typed navigate intent, dispatched by its own owner.
    Navigate(Navigation),
    /// The Left / Right menu: how the active panel shows its listing.
    Panel(PanelCommand),
    /// The Files menu rows that F-keys alone did not already cover.
    File(FileCommand),
    /// The Commands menu: everything that is not about one file.
    Tool(ToolCommand),
    /// A menu row Far gives no key. `resolve` never returns this — only the
    /// menu builds it — but it takes the same road as every other action, so
    /// a keyless row cannot behave differently from a keyed one.
    Menu(MenuCommand),
    /// `Ctrl+C` — leave at once, without asking. The emergency exit stays an
    /// emergency exit: a dialog in front of it would be one keystroke between
    /// the user and a program they have decided to stop.
    ///
    /// Bare `q` is deliberately *not* here: a printable key now starts a
    /// command, and a command line you cannot type `q` into cannot run `quit`,
    /// `qemu` or `queue`.
    Quit,
    /// `Left` / `Right` — make that side the active panel, or, in a
    /// multi-column view, move the cursor one column that way.
    SelectPanel(PanelSide),
    /// `Alt+F1` / `Alt+F2` — open the drive menu for that side.
    DriveMenu(PanelSide),
    /// `F3` — view the selected file.
    View,
    /// `F4` — edit the selected file in the built-in editor.
    Edit,
    /// `Alt+F4` — edit the selected file in the external editor.
    EditExternal,
    /// `F5` — copy to the other panel.
    Copy,
    /// `Shift+F6` — rename the entry where it is.
    Rename,
    /// `F6` — move to the other panel.
    Move,
    /// `F8` — open the delete confirmation dialog.
    DeleteConfirm,
    /// `F11` — Far's Plugin commands list: MCP status, `NetBox`'s stored
    /// sessions and their import.
    PluginCommands,
    /// `Ctrl+O` — toggle console mode (hide panels).
    ToggleConsole,
    /// `Ctrl+R` — reload both panels.
    Refresh,
    /// `:` — open an empty command line.
    CommandLine,
    /// Any printable key — open the command line already holding that
    /// character, the way Far Manager and every shell start a command.
    CommandLineChar(char),
    /// `Alt+<character>` — quick-search: append the character to the query.
    QuickSearch(char),
    /// `Backspace` — drop the last quick-search character.
    QuickSearchBackspace,
    /// `Esc` or any unmapped key — cancel the quick search. Every other
    /// action ends it too; see [`Action::edits_quick_search`].
    QuickSearchClear,
}

impl Action {
    /// Whether this is one of the quick search's own keys — `Alt+<character>`
    /// or `Backspace`. Any other action ends the search before it acts, as
    /// Far's fast find closes on any key that is not part of it.
    #[must_use]
    pub const fn edits_quick_search(self) -> bool {
        matches!(self, Self::QuickSearch(_) | Self::QuickSearchBackspace)
    }
}

/// Translate a raw key event into the [`Action`] it triggers in the main view.
///
/// This is the single source of truth for main-view key bindings. It is total:
/// any unmapped key resolves to [`Action::QuickSearchClear`], matching the
/// original behaviour where stray keystrokes cancel an in-progress quick search.
///
/// Ordering mirrors the historical dispatch: modifier-qualified bindings
/// (`Alt+F1`, `Shift+F4`, `Ctrl+*`) are checked before their bare counterparts.
#[must_use]
pub const fn resolve(key: KeyEvent) -> Action {
    if let Some(command) = menu::resolve(key) {
        return Action::MenuCommand(command);
    }
    resolve_main(key)
}

const fn resolve_main(key: KeyEvent) -> Action {
    keymap! { key;
        // Quit is Ctrl+C, among the Ctrl combos below. Bare `q` used to be
        // here; it went away when printable keys started a command — see
        // `Action::Quit`.

        // Navigation
        KeyCode::Down => Action::NavDown,
        KeyCode::Up => Action::NavUp,
        KeyCode::Enter => Action::Open,
        KeyCode::Tab => Action::SwitchPanel,
        KeyCode::Left => Action::SelectPanel(PanelSide::Left),
        KeyCode::Right => Action::SelectPanel(PanelSide::Right),
        KeyCode::Home => Action::First,
        KeyCode::End => Action::Last,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,

        KeyCode::F(_) => resolve_function_key(key),

        // Marking. Far's group keys are the grey ones on the numeric pad;
        // terminals do not report which pad a key came from, so these are the
        // plain characters. Nothing is lost: a command starting with `+`, `-`
        // or `*` is not a thing, and `:` still opens the command line for one.
        // With Alt they are quick-search characters like any other, so a name
        // starting with `-` or `+` can be found.
        KeyCode::Insert | KeyCode::Char(' ') => Action::ToggleMark,
        KeyCode::Char('+') if without(ALT) => Action::SelectGroup,
        KeyCode::Char('-') if without(ALT) => Action::UnselectGroup,
        KeyCode::Char('*') if without(ALT) => Action::InvertSelection,

        // Ctrl combos. WSL wants both modifiers, so it goes first.
        KeyCode::Char('w' | 'W') if has(CONTROL + ALT) => Action::WslManagement,
        KeyCode::Char(c) if has(CONTROL) => resolve_control_char(c),
        KeyCode::Delete if has(ALT) => Action::File(FileCommand::Wipe),

        // Command line
        KeyCode::Char(':') => Action::CommandLine,

        // Quick search (Alt+character), and its editing keys.
        KeyCode::Backspace => Action::QuickSearchBackspace,

        // Any other printable key starts a command. This arm comes last among
        // the `Char` arms so every binding above — `Space` for marking, the
        // Ctrl combos, `:`, Alt+character quick search — keeps its meaning.
        // An unclaimed Control character must not become command text.
        KeyCode::Char(c) => resolve_character(c, key.modifiers),

        // `Esc` and any other key cancel an in-progress quick search.
        _ => Action::QuickSearchClear,
    }
}

const fn resolve_character(ch: char, modifiers: KeyModifiers) -> Action {
    if modifiers.contains(KeyModifiers::ALT) {
        Action::QuickSearch(ch)
    } else if modifiers.contains(KeyModifiers::CONTROL) {
        Action::QuickSearchClear
    } else {
        Action::CommandLineChar(ch)
    }
}

/// `Ctrl+letter`. Only the letters named here mean anything; an unclaimed
/// Control character must not become command text, so the rest cancel the
/// quick search the way any stray key does.
///
/// Ctrl+I is Tab on the wire and Ctrl+digit is not distinguishable in most
/// terminals; both still arrive from the menu, which builds the key event
/// itself.
const fn resolve_control_char(ch: char) -> Action {
    match ch {
        'c' => Action::Quit,
        '\\' => Action::GoRoot,
        'u' => Action::SwapPanels,
        'o' => Action::ToggleConsole,
        'r' => Action::Refresh,
        'l' => Action::Information,
        'm' => Action::RestoreSelection,
        'q' => Action::Panel(PanelCommand::QuickView),
        'n' => Action::Panel(PanelCommand::LongNames),
        'a' => Action::File(FileCommand::Attributes),
        'g' => Action::File(FileCommand::ApplyCommand),
        'z' => Action::File(FileCommand::Describe),
        'i' => Action::Tool(ToolCommand::Filter),
        'w' => Action::Tool(ToolCommand::Tasks),
        '0'..='9' => resolve_view_digit(ch),
        _ => Action::QuickSearchClear,
    }
}

/// `Ctrl+1` … `Ctrl+0`: Far's ten view modes, in the order of the digits.
const fn resolve_view_digit(digit: char) -> Action {
    let view = match digit {
        '1' => PanelView::Brief,
        '2' => PanelView::Medium,
        '3' => PanelView::Full,
        '4' => PanelView::Wide,
        '5' => PanelView::Detailed,
        '6' => PanelView::Descriptions,
        '7' => PanelView::LongDescriptions,
        '8' => PanelView::Owners,
        '9' => PanelView::Links,
        _ => PanelView::Alternative,
    };
    Action::PanelView(view)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_far_menu_keys;
