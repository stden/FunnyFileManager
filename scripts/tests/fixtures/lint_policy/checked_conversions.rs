//! Positive: checked replacements for the rejected casts.

/// Saturates at the largest `u16`, like a coordinate clamped to the screen.
#[must_use]
pub fn narrow(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Maps negative input to zero instead of wrapping it.
#[must_use]
pub fn unsigned(value: i32) -> u32 {
    u32::try_from(value).unwrap_or(0)
}

/// Every `u32` is exactly representable as `f64`.
#[must_use]
pub fn ratio(value: u32) -> f64 {
    f64::from(value)
}
