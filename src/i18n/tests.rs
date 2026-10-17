//! Unit tests: a missing translation falls back to the English text.

use super::text;
use crate::config::Lang;

#[test]
fn a_language_without_translations_shows_the_english_text() {
    assert_eq!(text(Lang::En, "Files"), "Files");
    assert_eq!(text(Lang::Ru, "Files"), "Files", "never blank");
}
