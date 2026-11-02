//! Purpose: The screens list (`F12`): the screens this program has — panels and console.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// Which screen carries the cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenList {
    pub cursor: usize,
}
