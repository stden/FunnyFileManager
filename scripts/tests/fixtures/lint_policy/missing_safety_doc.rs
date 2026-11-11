//! Negative: a public `unsafe fn` whose caller contract is undocumented.

/// Reads one byte.
#[must_use]
#[allow(unsafe_code, reason = "the fixture needs an unsafe fn")]
pub const unsafe fn read(pointer: *const u8) -> u8 {
    // SAFETY: the caller guarantees `pointer` is valid for reads.
    unsafe { *pointer }
}
