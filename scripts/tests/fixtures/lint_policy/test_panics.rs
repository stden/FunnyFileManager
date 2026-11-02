//! Positive: clippy.toml lets tests unwrap, expect, index, panic, print and dbg!.

#[must_use]
pub fn parsed(text: &str) -> Option<u8> {
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::parsed;

    #[test]
    fn parses_digits() {
        let digits = [parsed("7").unwrap(), parsed("8").expect("a digit")];
        let slice = digits.as_slice();
        println!("parsed {}", slice[0]);
        dbg!(slice[1]);
        assert_eq!(digits, [7, 8]);
    }

    #[test]
    #[should_panic(expected = "no digit")]
    fn text_is_no_digit() {
        let digit = parsed("x").unwrap_or_else(|| panic!("no digit"));
        assert_eq!(digit, 0);
    }
}
