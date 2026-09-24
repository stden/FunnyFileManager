//! Negative: identifiers a later edition or a reader's font gets wrong, and import noise.

use std::cmp::{max};

/// `gen` is a keyword in edition 2024.
#[must_use]
pub fn gen(left: u8, right: u8) -> u8 {
    max(left, right)
}

/// Looks like `ETE` in some fonts and cannot be typed on others.
pub const ÉTÉ: u8 = 1;
