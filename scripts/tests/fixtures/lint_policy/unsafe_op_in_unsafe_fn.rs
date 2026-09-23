//! Negative: an unsafe operation in an `unsafe fn` body, outside an `unsafe` block.

/// Reads one byte.
///
/// # Safety
///
/// `pointer` must be valid for reads.
#[must_use]
#[allow(unsafe_code)]
pub const unsafe fn read(pointer: *const u8) -> u8 {
    *pointer
}
