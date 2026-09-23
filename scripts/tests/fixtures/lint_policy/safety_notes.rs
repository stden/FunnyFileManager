//! Negative: safety notes where nothing is unsafe, and an allowance with no reason.

/// Doubles a byte.
///
/// # Safety
///
/// Nothing to uphold.
#[must_use]
pub const fn doubled(value: u8) -> u8 {
    value.wrapping_mul(2)
}

// SAFETY: nothing here is unsafe.
pub const ONE: u8 = 1;

#[allow(dead_code)]
const UNUSED: u8 = 0;
