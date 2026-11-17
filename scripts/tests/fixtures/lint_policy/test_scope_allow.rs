//! Positive: test code may use the crate root's `cfg(test)` allowances.

#[must_use]
pub const fn doubled(value: u8) -> u8 {
    value.wrapping_mul(2)
}

#[cfg(test)]
mod tests {
    use super::doubled;

    fn checked(value: u8) -> Result<u8, String> {
        Ok(doubled(value))
    }

    #[test]
    fn doubles() {
        assert_eq!(checked(2), Ok(4));
    }
}
