//! Negative: two unsafe operations under one SAFETY comment, and an unsafe
//! attribute spelled as if it were safe.

/// Adds the bytes behind two pointers.
///
/// # Safety
///
/// Both pointers must be valid for reads.
#[must_use]
#[allow(unsafe_code, reason = "the fixture needs an unsafe block")]
pub const unsafe fn sum(left: *const u8, right: *const u8) -> u8 {
    // SAFETY: the caller guarantees both pointers are valid for reads.
    unsafe { (*left).wrapping_add(*right) }
}

/// Exported under its own name: a clash with another symbol is undefined behaviour.
#[allow(unsafe_code, reason = "the fixture needs an unsafe attribute")]
#[no_mangle]
pub static LINT_POLICY_FIXTURE: u8 = 0;
