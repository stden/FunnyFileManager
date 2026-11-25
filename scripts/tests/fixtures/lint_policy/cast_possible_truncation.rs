//! Negative: an `as` cast that silently drops the high bits of a `u32`.

#[must_use]
pub const fn narrow(value: u32) -> u16 {
    value as u16
}
