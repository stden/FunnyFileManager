//! Negative: production code wraps a value that can never be absent.
//!
//! The crate root allows this lint only under `cfg(test)`, so the library
//! build must still reject it.

const fn doubled(value: u8) -> Option<u8> {
    Some(value.wrapping_mul(2))
}

#[must_use]
pub fn doubled_or_zero(value: u8) -> u8 {
    doubled(value).unwrap_or_default()
}
