//! Purpose: A history shown as a list — commands (`Alt+F8`), viewed files (`Alt+F11`) or folders (`Alt+F12`). One type, because the three lists are walked the same way.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// The lines of a history, newest last, and where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryList {
    pub entries: Vec<String>,
    pub cursor: usize,
}
