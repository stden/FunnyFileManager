//! Purpose: The keys of the task list's search bar (`/`), one pure mapping
//! from a key to a [`TaskSearchCommand`].
//!
//! The bar's editing keys are the shared one-line ones of every prompt, and
//! a list key closes the bar and moves; only the one key the bar adds is
//! here. `popup::tasks::bars` applies it. Resolving a key has no effect.

use crossterm::event::{KeyCode, KeyEvent};

/// What one key asks of the open search bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSearchCommand {
    /// F3: the next match of what is typed, after the cursor.
    FindNext,
}

/// What `key` asks of the search bar; `None` for a key it does not add.
#[must_use]
pub const fn resolve(key: KeyEvent) -> Option<TaskSearchCommand> {
    match key.code {
        KeyCode::F(3) => Some(TaskSearchCommand::FindNext),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
