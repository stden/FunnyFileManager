//! Purpose: The task list (`Ctrl+W`): the processes this program started that are still running.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// The tasks on screen and where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskList {
    pub cursor: usize,
}
