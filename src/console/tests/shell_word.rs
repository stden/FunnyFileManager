//! Purpose: Pins shell quoting: a plain word as it is, anything else quoted
//! the platform's way, and the Windows rule on every platform.

use super::super::quoting::windows_word;
use super::*;

#[test]
fn a_plain_word_is_its_own_shell_word() {
    assert_eq!(shell_word("--scope"), "--scope");
    assert_eq!(shell_word("/srv/x/ffm-mcp"), "/srv/x/ffm-mcp");
}

#[cfg(unix)]
#[test]
fn anything_else_is_single_quoted_on_unix() {
    assert_eq!(shell_word(""), "''");
    assert_eq!(shell_word("my tools"), "'my tools'");
    assert_eq!(shell_word("it's"), r"'it'\''s'");
    assert_eq!(shell_word(r#"{"command":"/x"}"#), r#"'{"command":"/x"}'"#);
}

/// The `claude mcp add-json` argument a Windows user is told to type: the
/// C runtime must hand the JSON back exactly, backslashes and quotes alike.
#[test]
fn a_windows_word_survives_the_c_runtime_split() {
    assert_eq!(windows_word("my tools"), r#""my tools""#);
    assert_eq!(
        windows_word(r#"{"command":"C:\\x"}"#),
        r#""{\"command\":\"C:\\x\"}""#
    );
    // Backslashes before a quote, and before the closing one, are doubled.
    assert_eq!(windows_word(r#"a\"b"#), r#""a\\\"b""#);
    assert_eq!(windows_word(r"C:\dir\"), r#""C:\dir\\""#);
}
