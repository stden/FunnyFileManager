//! A documented `unsafe` block with no item-level opt-in.
//!
//! Rejected where the crate denies or forbids `unsafe_code`; accepted in the
//! FFI crate, which allows unsafe code crate-wide.

#[must_use]
pub const fn first(value: &u8) -> u8 {
    let pointer = std::ptr::from_ref(value);
    // SAFETY: `pointer` comes from a live shared reference.
    unsafe { *pointer }
}
