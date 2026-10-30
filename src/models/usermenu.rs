//! Purpose: The user menu (`F2`): the commands of `menu.ini`, shown as a list.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// The menu on screen and where the cursor is in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserMenu {
    pub cursor: usize,
}
