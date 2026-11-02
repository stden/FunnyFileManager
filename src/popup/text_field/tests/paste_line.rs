//! Purpose: A pasted block becomes one line, with the breaks turned into spaces.

use super::super::one_line;

#[test]
fn a_pasted_text_becomes_one_line() {
    assert_eq!(one_line("a\r\nb\nc\n"), "a b c");
    assert_eq!(one_line("plain"), "plain");
    assert_eq!(one_line("\n\n"), "");
}
