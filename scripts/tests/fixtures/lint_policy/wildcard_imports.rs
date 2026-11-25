//! Negative: a glob import hides where each name comes from.

use std::cmp::*;

#[must_use]
pub fn larger(left: u8, right: u8) -> u8 {
    max(left, right)
}
