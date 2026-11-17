//! Negative: an opted-in `unsafe` block without the `SAFETY` comment that justifies it.

#[must_use]
#[allow(unsafe_code, reason = "the fixture needs an unsafe block")]
pub const fn first(value: &u8) -> u8 {
    let pointer = std::ptr::from_ref(value);
    unsafe { *pointer }
}
