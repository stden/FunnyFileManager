//! Negative: an `as` cast that turns a negative `i32` into a huge `u32`.

#[must_use]
pub const fn unsigned(value: i32) -> u32 {
    value as u32
}
