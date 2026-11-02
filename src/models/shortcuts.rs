//! Purpose: The folder shortcuts dialog: the ten bookmark slots as a list.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// Which slot carries the cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortcutList {
    pub cursor: usize,
}
