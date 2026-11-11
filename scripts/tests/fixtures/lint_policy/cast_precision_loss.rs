//! Negative: an `as` cast that rounds large `u64` values once they become `f64`.

#[must_use]
pub const fn ratio(value: u64) -> f64 {
    value as f64
}
