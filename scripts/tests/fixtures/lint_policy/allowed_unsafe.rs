//! The sanctioned shape: a narrow item-level opt-in plus a `SAFETY` comment.
//!
//! Accepted where `unsafe_code` is denied; rejected where it is forbidden,
//! because `forbid` cannot be lowered by an `allow` attribute.

#[must_use]
#[allow(unsafe_code)]
pub const fn first(value: &u8) -> u8 {
    let pointer = std::ptr::from_ref(value);
    // SAFETY: `pointer` comes from a live shared reference.
    unsafe { *pointer }
}

/// Reads one byte.
///
/// # Safety
///
/// `pointer` must be valid for reads.
#[must_use]
#[allow(unsafe_code)]
pub const unsafe fn read(pointer: *const u8) -> u8 {
    // SAFETY: the caller upholds this function's contract.
    unsafe { *pointer }
}
