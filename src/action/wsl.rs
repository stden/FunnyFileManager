//! WSL dialog key language. Resolving input never starts a process.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslCommand {
    Up,
    Down,
    First,
    Last,
    Open,
    Refresh,
    Update,
    UpdatePackages,
    Close,
    Ignore,
}

/// Resolve one key to a dialog command. System and package updates stay distinct.
#[must_use]
pub fn resolve(key: KeyEvent) -> WslCommand {
    use WslCommand::{Close, Down, First, Ignore, Last, Open, Refresh, Up, Update, UpdatePackages};

    keymap! { key;
        KeyCode::Up => Up,
        KeyCode::Down => Down,
        KeyCode::Home => First,
        KeyCode::End => Last,
        KeyCode::Enter if without(CONTROL + ALT) => Open,
        KeyCode::F(2) if without(CONTROL + ALT + SHIFT) => Refresh,
        KeyCode::F(5) if without(CONTROL + ALT + SHIFT) => Update,
        KeyCode::F(6) if without(CONTROL + ALT + SHIFT) => UpdatePackages,
        KeyCode::Esc => Close,
        _ => Ignore,
    }
}

#[cfg(test)]
mod tests;
