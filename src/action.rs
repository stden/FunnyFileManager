//! Input → Action DSL.
//!
//! This module separates *what key was pressed* from *what the app does*. The
//! event loop in [`crate`] resolves a raw [`KeyEvent`] into a typed [`Action`]
//! via [`resolve`], then a dispatcher applies it. Keeping the keymap in one
//! pure, exhaustively-testable function (no terminal, no I/O) mirrors the
//! approach [`crate::command`] already uses for the command line, and makes the
//! full set of main-view bindings reviewable at a glance.
//!
//! Only *main-view* keys live here; modal/overlay keys are handled while an
//! [`crate::models::Overlay`] is active (see
//! [`crate::popup::process_popup_input`]).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::PanelSide;

/// Define a total key-to-action resolver from an ordered binding list.
///
/// The optional `has` clause requires every named modifier while allowing
/// extras. Unqualified bindings deliberately ignore modifiers, and the first
/// matching arm wins.
macro_rules! keymap {
    (
        $key:ident;
        $(
            $code:pat $(if has($first:ident $(+ $rest:ident)*))? => $action:expr,
        )+
    ) => {
        match $key.code {
            $(
                $code $(if $key.modifiers.contains(KeyModifiers::$first)
                    $(&& $key.modifiers.contains(KeyModifiers::$rest))*)?
                    => $action,
            )+
        }
    };
}

/// A single user-triggerable operation in the main dual-panel view.
///
/// Every variant corresponds to exactly one effect applied to the
/// [`App`](crate::models::App); the mapping from keys to actions is [`resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `F10` / `Ctrl+C` — leave the application.
    ///
    /// Bare `q` is deliberately *not* here: a printable key now starts a
    /// command, and a command line you cannot type `q` into cannot run `quit`,
    /// `qemu` or `queue`.
    Quit,
    /// `Down` — move the selection down (subject to nav debounce).
    NavDown,
    /// `Up` — move the selection up (subject to nav debounce).
    NavUp,
    /// `Enter` — open the selected directory / file.
    Open,
    /// `Tab` — switch the active panel.
    SwitchPanel,
    /// `Left` / `Right` — make that side the active panel.
    SelectPanel(PanelSide),
    /// `Home` — jump to the first entry.
    First,
    /// `End` — jump to the last entry.
    Last,
    /// `PgUp` — move the selection up one screen.
    PageUp,
    /// `PgDn` — move the selection down one screen.
    PageDown,
    /// `Alt+F1` / `Alt+F2` — open the drive menu for that side.
    DriveMenu(PanelSide),
    /// `Shift+F4` — open the "create new file" dialog.
    NewFile,
    /// `F1` — toggle the help overlay.
    ToggleHelp,
    /// `F3` — view the selected file.
    View,
    /// `F4` — edit the selected file in the built-in editor.
    Edit,
    /// `Alt+F4` — edit the selected file in the external editor.
    EditExternal,
    /// `F5` — copy to the other panel.
    Copy,
    /// `F6` — move to the other panel.
    Move,
    /// `F7` — create a directory.
    Mkdir,
    /// `F8` — open the delete confirmation dialog.
    DeleteConfirm,
    /// `F9` — open the MCP status view.
    McpStatus,
    /// `Insert` / `Space` — toggle the mark on the selection and advance.
    ToggleMark,
    /// `Ctrl+\` — jump to the filesystem root.
    GoRoot,
    /// `Ctrl+U` — swap the contents of the two panels.
    SwapPanels,
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
    /// `Esc` or any other key — cancel the quick search.
    QuickSearchClear,
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
    keymap! { key;
        // Quit. Bare `q` used to be here; it went away when printable keys
        // started a command — see `Action::Quit`.
        KeyCode::F(10) => Action::Quit,
        KeyCode::Char('c') if has(CONTROL) => Action::Quit,

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

        // Drive menus must precede bare F1.
        KeyCode::F(1) if has(ALT) => Action::DriveMenu(PanelSide::Left),
        KeyCode::F(2) if has(ALT) => Action::DriveMenu(PanelSide::Right),
        // External editor and new file must precede bare F4.
        KeyCode::F(4) if has(ALT) => Action::EditExternal,
        KeyCode::F(4) if has(SHIFT) => Action::NewFile,

        // F-key commands
        KeyCode::F(1) => Action::ToggleHelp,
        KeyCode::F(3) => Action::View,
        KeyCode::F(4) => Action::Edit,
        KeyCode::F(5) => Action::Copy,
        KeyCode::F(6) => Action::Move,
        KeyCode::F(7) => Action::Mkdir,
        KeyCode::F(8) => Action::DeleteConfirm,
        KeyCode::F(9) => Action::McpStatus,

        // Marking
        KeyCode::Insert | KeyCode::Char(' ') => Action::ToggleMark,

        // Ctrl combos
        KeyCode::Char('\\') if has(CONTROL) => Action::GoRoot,
        KeyCode::Char('u') if has(CONTROL) => Action::SwapPanels,
        KeyCode::Char('o') if has(CONTROL) => Action::ToggleConsole,
        KeyCode::Char('r') if has(CONTROL) => Action::Refresh,

        // Command line
        KeyCode::Char(':') => Action::CommandLine,

        // Quick search (Alt+character), and its editing keys.
        KeyCode::Char(c) if has(ALT) => Action::QuickSearch(c),
        KeyCode::Backspace => Action::QuickSearchBackspace,

        // Any other printable key starts a command. This arm comes last among
        // the `Char` arms so every binding above — `Space` for marking, the
        // Ctrl combos, `:`, Alt+character quick search — keeps its meaning.
        // An unclaimed Control character must not become command text.
        KeyCode::Char(_) if has(CONTROL) => Action::QuickSearchClear,
        KeyCode::Char(c) => Action::CommandLineChar(c),

        // `Esc` and any other key cancel an in-progress quick search.
        _ => Action::QuickSearchClear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }
    fn key_mod(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, m)
    }

    #[test]
    fn test_resolve_quit_variants() {
        assert_eq!(resolve(key(KeyCode::F(10))), Action::Quit);
        assert_eq!(
            resolve(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
    }

    /// Bare `q` starts the command `quit` instead of quitting outright — a
    /// command line you cannot type `q` into cannot run `quit` or `qemu`.
    #[test]
    fn test_resolve_bare_q_types_instead_of_quitting() {
        assert_eq!(
            resolve(key(KeyCode::Char('q'))),
            Action::CommandLineChar('q')
        );
        assert_eq!(
            resolve(key(KeyCode::Char('Q'))),
            Action::CommandLineChar('Q')
        );
    }

    /// Every printable key opens the prompt, except the ones the main view
    /// still claims: `Space` marks, `:` opens an empty prompt, Ctrl combos and
    /// Alt+character keep their own meanings.
    #[test]
    fn test_resolve_printable_keys_start_a_command() {
        assert_eq!(
            resolve(key(KeyCode::Char('l'))),
            Action::CommandLineChar('l')
        );
        assert_eq!(
            resolve(key(KeyCode::Char('/'))),
            Action::CommandLineChar('/')
        );
        assert_eq!(resolve(key(KeyCode::Char(' '))), Action::ToggleMark);
        assert_eq!(resolve(key(KeyCode::Char(':'))), Action::CommandLine);
        assert_eq!(
            resolve(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Action::SwapPanels
        );
        assert_eq!(
            resolve(key_mod(KeyCode::Char('a'), KeyModifiers::ALT)),
            Action::QuickSearch('a')
        );
    }

    #[test]
    fn test_resolve_navigation() {
        assert_eq!(resolve(key(KeyCode::Down)), Action::NavDown);
        assert_eq!(resolve(key(KeyCode::Up)), Action::NavUp);
        assert_eq!(resolve(key(KeyCode::Enter)), Action::Open);
        assert_eq!(resolve(key(KeyCode::Tab)), Action::SwitchPanel);
        assert_eq!(
            resolve(key(KeyCode::Left)),
            Action::SelectPanel(PanelSide::Left)
        );
        assert_eq!(
            resolve(key(KeyCode::Right)),
            Action::SelectPanel(PanelSide::Right)
        );
        assert_eq!(resolve(key(KeyCode::Home)), Action::First);
        assert_eq!(resolve(key(KeyCode::End)), Action::Last);
    }

    #[test]
    fn test_resolve_paging() {
        assert_eq!(resolve(key(KeyCode::PageUp)), Action::PageUp);
        assert_eq!(resolve(key(KeyCode::PageDown)), Action::PageDown);
    }

    #[test]
    fn test_resolve_fkeys() {
        assert_eq!(resolve(key(KeyCode::F(1))), Action::ToggleHelp);
        assert_eq!(resolve(key(KeyCode::F(3))), Action::View);
        assert_eq!(resolve(key(KeyCode::F(4))), Action::Edit);
        assert_eq!(resolve(key(KeyCode::F(5))), Action::Copy);
        assert_eq!(resolve(key(KeyCode::F(6))), Action::Move);
        assert_eq!(resolve(key(KeyCode::F(7))), Action::Mkdir);
        assert_eq!(resolve(key(KeyCode::F(8))), Action::DeleteConfirm);
        assert_eq!(resolve(key(KeyCode::F(9))), Action::McpStatus);
    }

    #[test]
    fn test_resolve_modifier_qualified_precede_bare() {
        // Alt+F1/F2 are drive menus, not help.
        assert_eq!(
            resolve(key_mod(KeyCode::F(1), KeyModifiers::ALT)),
            Action::DriveMenu(PanelSide::Left)
        );
        assert_eq!(
            resolve(key_mod(KeyCode::F(2), KeyModifiers::ALT)),
            Action::DriveMenu(PanelSide::Right)
        );
        // Shift+F4 is "new file", Alt+F4 is "external editor" — not bare "edit".
        assert_eq!(
            resolve(key_mod(KeyCode::F(4), KeyModifiers::SHIFT)),
            Action::NewFile
        );
        assert_eq!(
            resolve(key_mod(KeyCode::F(4), KeyModifiers::ALT)),
            Action::EditExternal
        );
    }

    #[test]
    fn test_resolve_dsl_preserves_order_and_modifier_containment() {
        let cases = [
            (
                KeyCode::F(4),
                KeyModifiers::ALT | KeyModifiers::SHIFT,
                Action::EditExternal,
            ),
            (
                KeyCode::F(1),
                KeyModifiers::ALT | KeyModifiers::CONTROL,
                Action::DriveMenu(PanelSide::Left),
            ),
            (
                KeyCode::Char('u'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
                Action::SwapPanels,
            ),
            (
                KeyCode::Char('c'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
                Action::Quit,
            ),
            (KeyCode::F(5), KeyModifiers::CONTROL, Action::Copy),
            (KeyCode::Enter, KeyModifiers::ALT, Action::Open),
            (KeyCode::Char(' '), KeyModifiers::ALT, Action::ToggleMark),
            (KeyCode::Char(':'), KeyModifiers::ALT, Action::CommandLine),
            (
                KeyCode::Backspace,
                KeyModifiers::SHIFT,
                Action::QuickSearchBackspace,
            ),
            (
                KeyCode::F(2),
                KeyModifiers::empty(),
                Action::QuickSearchClear,
            ),
            (
                KeyCode::Char('C'),
                KeyModifiers::CONTROL,
                Action::QuickSearchClear,
            ),
        ];

        for (code, modifiers, expected) in cases {
            assert_eq!(resolve(key_mod(code, modifiers)), expected);
        }
    }

    #[test]
    fn test_resolve_ctrl_combos() {
        assert_eq!(
            resolve(key_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL)),
            Action::GoRoot
        );
        assert_eq!(
            resolve(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Action::SwapPanels
        );
        assert_eq!(
            resolve(key_mod(KeyCode::Char('o'), KeyModifiers::CONTROL)),
            Action::ToggleConsole
        );
        assert_eq!(
            resolve(key_mod(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Action::Refresh
        );
    }

    #[test]
    fn test_resolve_marking_and_command_line() {
        assert_eq!(resolve(key(KeyCode::Insert)), Action::ToggleMark);
        assert_eq!(resolve(key(KeyCode::Char(' '))), Action::ToggleMark);
        assert_eq!(resolve(key(KeyCode::Char(':'))), Action::CommandLine);
    }

    #[test]
    fn test_resolve_quick_search() {
        assert_eq!(
            resolve(key_mod(KeyCode::Char('a'), KeyModifiers::ALT)),
            Action::QuickSearch('a')
        );
        assert_eq!(
            resolve(key(KeyCode::Backspace)),
            Action::QuickSearchBackspace
        );
        assert_eq!(resolve(key(KeyCode::Esc)), Action::QuickSearchClear);
        // A printable key no longer clears the search — it starts a command.
        assert_eq!(
            resolve(key(KeyCode::Char('z'))),
            Action::CommandLineChar('z')
        );
        assert_eq!(resolve(key(KeyCode::Delete)), Action::QuickSearchClear);
    }

    #[test]
    fn test_resolve_quick_search_accepts_any_unclaimed_alt_character() {
        for character in ['A', '7', '?', 'λ'] {
            assert_eq!(
                resolve(key_mod(KeyCode::Char(character), KeyModifiers::ALT)),
                Action::QuickSearch(character)
            );
        }
    }

    #[test]
    fn test_alt_q_is_quick_search_not_quit() {
        // `q` no longer quits at all, so Alt+q is nothing but quick search.
        assert_eq!(
            resolve(key_mod(KeyCode::Char('q'), KeyModifiers::ALT)),
            Action::QuickSearch('q')
        );
    }
}
