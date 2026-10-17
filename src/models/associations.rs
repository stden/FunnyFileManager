//! Purpose: The file associations dialog: which command opens which mask.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// The associations on screen and where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssociationList {
    pub cursor: usize,
}
