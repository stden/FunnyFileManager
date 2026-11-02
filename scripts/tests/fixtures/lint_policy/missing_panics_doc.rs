//! Negative: a public function that can panic, without a `# Panics` section.

#[must_use]
pub fn digit(text: &str) -> u8 {
    text.parse().expect("a decimal digit")
}
