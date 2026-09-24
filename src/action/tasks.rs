//! Purpose: The task list's keys (`Ctrl+W`): one pure mapping from a key to
//! a [`TaskCommand`], and the rows of the list's help page.
//!
//! Inside the list a key keeps the meaning Far or ffm gives it wherever that
//! meaning fits a process list; htop's meaning fills the keys Far leaves
//! empty; and every htop key that loses lands on a harmless dialog, so no
//! reflex from either program ends a process without a second, explicit
//! step. Resolving a key has no effect: `popup::tasks` applies the command.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{ListMotion, MaskPurpose, NiceStep, SortKey};
use crate::scroll::WIDE_STEP;

/// What one key asks of the task list while nothing is open on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCommand {
    /// Up, Down, `PgUp`, `PgDn`, Home, End.
    Move(ListMotion),
    /// Left, Right, Ctrl+Left, Ctrl+Right: the Command column sideways.
    Pan(Pan),
    /// Space, Insert: mark or unmark the process, then move down.
    ToggleMark,
    /// `c`: mark the process and everything descending from it.
    MarkFamily,
    /// `U`.
    UnmarkAll,
    /// `*`: invert the marks of the shown rows.
    InvertMarks,
    /// Gray `+` and `-`: the Select or Deselect dialog, a Far mask over
    /// process names.
    MarkByMask(MaskPurpose),
    /// Enter, F3: the Process information dialog.
    Info,
    /// F1, `h`, `?`.
    Help,
    /// F2: the command line or the name in the last column.
    ToggleCommandView,
    /// F4, `\`: the filter bar.
    Filter,
    /// `/` opens the search bar empty; Alt+character opens it with that
    /// character typed.
    Search(Option<char>),
    /// A digit starts a PID jump with that digit typed.
    PidJump(char),
    /// F5, `t`, Ctrl+T: the list or the parent tree.
    ToggleTree,
    /// F6, Ctrl+F12, `<`, `>`: the Sort by menu.
    SortMenu,
    /// A direct sort key; the key already in force reverses the order.
    SortBy(SortKey),
    /// `I`: the same key the other way round.
    InvertOrder,
    /// `u`: the Show processes of menu.
    UserMenu,
    /// `K`: kernel threads shown or hidden.
    ToggleKernelThreads,
    /// F7: the Priority form.
    Priority,
    /// `]` raises the priority one step at once; `[` lowers it, at once for
    /// root and after a confirmation for anyone else, who cannot undo it.
    Nice(NiceStep),
    /// F8, Del: the Terminate confirmation. Never a signal by itself.
    Terminate,
    /// F9, `k`: the Send signal menu, then the same confirmation.
    SignalMenu,
    /// F11, `Z`: hold or resume the refresh.
    ToggleHold,
    /// Ctrl+R, Ctrl+L: one sample now.
    RefreshNow,
    /// Esc, F10, `q`, Ctrl+C, Ctrl+W: close the list. Never quits ffm.
    Close,
    /// Every other key: consumed, so nothing falls through to the panels.
    Ignore,
}

/// A sideways scroll of the last column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pan {
    Left(PanStep),
    Right(PanStep),
}

/// How far one pan goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanStep {
    /// Left, Right.
    Cell,
    /// Ctrl+Left, Ctrl+Right: the F3 viewer's wide step.
    Wide,
}

impl PanStep {
    /// The cells this step pans by.
    #[must_use]
    pub const fn cells(self) -> usize {
        match self {
            Self::Cell => 1,
            Self::Wide => WIDE_STEP,
        }
    }
}

/// The key as the task list reads it: a shifted character is its capital
/// with no SHIFT, however the terminal sent it — `k` with SHIFT, or `K` with
/// or without it — so `K` is never taken for `k`.
#[must_use]
pub const fn normalize(key: KeyEvent) -> KeyEvent {
    match key.code {
        KeyCode::Char(ch) if key.modifiers.contains(KeyModifiers::SHIFT) => KeyEvent {
            code: KeyCode::Char(ch.to_ascii_uppercase()),
            modifiers: key.modifiers.difference(KeyModifiers::SHIFT),
            ..key
        },
        _ => key,
    }
}

/// What `key` asks of the task list in Browse mode.
#[must_use]
pub const fn resolve(key: KeyEvent) -> TaskCommand {
    let key = normalize(key);
    keymap! { key;
        KeyCode::Char(ch) if has(CONTROL) => control(ch),
        KeyCode::Char(ch) if has(ALT) => TaskCommand::Search(Some(ch)),
        KeyCode::Char(ch) => character(ch),
        KeyCode::F(number) if exact(CONTROL) => control_function_key(number),
        KeyCode::F(number) if without(CONTROL + ALT + SHIFT) => function_key(number),
        _ => named(key),
    }
}

/// Ctrl+letter. Ctrl+C closes the list rather than quitting ffm.
const fn control(ch: char) -> TaskCommand {
    match ch {
        'c' | 'C' | 'w' | 'W' => TaskCommand::Close,
        't' | 'T' => TaskCommand::ToggleTree,
        'r' | 'R' | 'l' | 'L' => TaskCommand::RefreshNow,
        _ => TaskCommand::Ignore,
    }
}

/// A character without Control or Alt: marking, finding and closing.
const fn character(ch: char) -> TaskCommand {
    match ch {
        ' ' => TaskCommand::ToggleMark,
        'c' => TaskCommand::MarkFamily,
        'U' => TaskCommand::UnmarkAll,
        '*' => TaskCommand::InvertMarks,
        '+' => TaskCommand::MarkByMask(MaskPurpose::Select),
        '-' => TaskCommand::MarkByMask(MaskPurpose::Unselect),
        '\\' => TaskCommand::Filter,
        '/' => TaskCommand::Search(None),
        '0'..='9' => TaskCommand::PidJump(ch),
        'h' | '?' => TaskCommand::Help,
        'q' => TaskCommand::Close,
        _ => htop_letter(ch),
    }
}

/// htop's letters for what is shown, its order, the refresh, and the
/// processes themselves.
const fn htop_letter(ch: char) -> TaskCommand {
    match ch {
        't' => TaskCommand::ToggleTree,
        'u' => TaskCommand::UserMenu,
        'K' => TaskCommand::ToggleKernelThreads,
        '<' | '>' => TaskCommand::SortMenu,
        'P' => TaskCommand::SortBy(SortKey::Cpu),
        'M' => TaskCommand::SortBy(SortKey::Resident),
        'T' => TaskCommand::SortBy(SortKey::CpuTime),
        'N' => TaskCommand::SortBy(SortKey::Pid),
        'I' => TaskCommand::InvertOrder,
        'Z' => TaskCommand::ToggleHold,
        ']' => TaskCommand::Nice(NiceStep::Raise),
        '[' => TaskCommand::Nice(NiceStep::Lower),
        'k' => TaskCommand::SignalMenu,
        _ => TaskCommand::Ignore,
    }
}

/// A bare F-key: the key bar's labels.
const fn function_key(number: u8) -> TaskCommand {
    match number {
        1 => TaskCommand::Help,
        2 => TaskCommand::ToggleCommandView,
        3 => TaskCommand::Info,
        4 => TaskCommand::Filter,
        5 => TaskCommand::ToggleTree,
        6 => TaskCommand::SortMenu,
        7 => TaskCommand::Priority,
        8 => TaskCommand::Terminate,
        9 => TaskCommand::SignalMenu,
        10 => TaskCommand::Close,
        11 => TaskCommand::ToggleHold,
        _ => TaskCommand::Ignore,
    }
}

/// Ctrl+F-key: the panel's direct sort keys, each on the process column
/// that answers the same question. Extension (Ctrl+F4), modification time
/// (Ctrl+F5) and access time (Ctrl+F9) have no process analogue.
const fn control_function_key(number: u8) -> TaskCommand {
    match number {
        3 => TaskCommand::SortBy(SortKey::Name),
        6 => TaskCommand::SortBy(SortKey::Resident),
        7 => TaskCommand::SortBy(SortKey::Pid),
        8 => TaskCommand::SortBy(SortKey::Started),
        10 => TaskCommand::SortBy(SortKey::Command),
        11 => TaskCommand::SortBy(SortKey::User),
        12 => TaskCommand::SortMenu,
        _ => TaskCommand::Ignore,
    }
}

/// The keys with a name rather than a character.
const fn named(key: KeyEvent) -> TaskCommand {
    keymap! { key;
        KeyCode::Up => TaskCommand::Move(ListMotion::Up),
        KeyCode::Down => TaskCommand::Move(ListMotion::Down),
        KeyCode::PageUp => TaskCommand::Move(ListMotion::PageUp),
        KeyCode::PageDown => TaskCommand::Move(ListMotion::PageDown),
        KeyCode::Home => TaskCommand::Move(ListMotion::Home),
        KeyCode::End => TaskCommand::Move(ListMotion::End),
        KeyCode::Left if has(CONTROL) => TaskCommand::Pan(Pan::Left(PanStep::Wide)),
        KeyCode::Right if has(CONTROL) => TaskCommand::Pan(Pan::Right(PanStep::Wide)),
        KeyCode::Left => TaskCommand::Pan(Pan::Left(PanStep::Cell)),
        KeyCode::Right => TaskCommand::Pan(Pan::Right(PanStep::Cell)),
        KeyCode::Insert => TaskCommand::ToggleMark,
        KeyCode::Enter => TaskCommand::Info,
        KeyCode::Delete if without(CONTROL + ALT + SHIFT) => TaskCommand::Terminate,
        KeyCode::Esc => TaskCommand::Close,
        _ => TaskCommand::Ignore,
    }
}

/// The help page (`F1`): each row's keys, as Far writes them and separated by
/// `", "`, then what they do. Every key named here resolves to a command.
pub const HELP: &[(&str, &str)] = &[
    (
        "Up, Down, PgUp, PgDn",
        "Move the cursor; it stays on its process",
    ),
    ("Home, End", "First or last process"),
    ("Left, Right", "Scroll the Command column"),
    ("Ctrl+Left, Ctrl+Right", "Scroll the Command column by 20"),
    ("Space, Ins", "Mark the process and move down"),
    ("c", "Mark the process and its descendants"),
    ("U", "Unmark all"),
    ("*", "Invert the marks"),
    ("+, -", "Select or deselect by a name mask"),
    ("Enter, F3", "Process information"),
    ("F2", "Command line or name in the last column"),
    ("F4, \\", "Filter: fixed strings, | means or"),
    ("/, Alt+letter", "Search; F3 in the bar finds the next"),
    ("0-9", "Jump to a PID"),
    ("F5, t, Ctrl+T", "List or tree"),
    ("F6, Ctrl+F12, <, >", "Sort by menu"),
    ("P, M, T, N", "Sort by CPU%, memory, CPU time, PID"),
    ("Ctrl+F3, Ctrl+F6, Ctrl+F7", "Sort by name, memory, PID"),
    (
        "Ctrl+F8, Ctrl+F10, Ctrl+F11",
        "Sort by start, command, user",
    ),
    ("I", "Invert the order"),
    ("u", "Show the processes of one user"),
    ("K", "Show or hide kernel threads"),
    ("F7", "Set the nice value of the main thread"),
    ("]", "Nice -1, at once: runs sooner; needs root"),
    ("[", "Nice +1: runs later; asks unless root"),
    ("F8, Del", "Terminate or kill, after a confirmation"),
    ("F9, k", "Send a signal, after a confirmation"),
    ("F11, Z", "Hold or resume the refresh"),
    ("Ctrl+R, Ctrl+L", "Refresh now"),
    ("F1, h, ?", "This help"),
    ("Esc, F10, q, Ctrl+C, Ctrl+W", "Close the task list"),
];

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_help;
