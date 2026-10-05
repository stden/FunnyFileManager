//! Purpose: the main view's own keymap — navigation, marking, the Ctrl combos, and the printable keys that start a command or a quick search.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{PanelSide, PanelView};

use super::function_keys::resolve_function_key;
use super::{Action, FileCommand, PanelCommand, ToolCommand};

pub(super) const fn resolve_main(key: KeyEvent) -> Action {
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

pub(super) const fn resolve_character(ch: char, modifiers: KeyModifiers) -> Action {
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
