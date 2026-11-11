//! Purpose: Pin the argument grammar — splitting, quoting, refusals, the
//! round trip through the form's line and the argument being typed.

use super::*;

fn arguments(line: &str) -> Result<Vec<String>, ValueError> {
    Arguments::parse_line(line).map(Vec::from)
}

#[test]
fn arguments_split_on_whitespace_and_read_quotes_anywhere() {
    let cases: [(&str, &[&str]); 8] = [
        ("", &[]),
        ("   ", &[]),
        ("--stdio", &["--stdio"]),
        ("  start   --stdio ", &["start", "--stdio"]),
        (r#""a b""#, &["a b"]),
        (r#"--x="a b""#, &["--x=a b"]),
        (r#"a""b"#, &["ab"]),
        (r#""a""b" """#, &["a\"b", ""]),
    ];
    for (line, expected) in cases {
        assert_eq!(
            arguments(line),
            Ok(expected.iter().map(|s| (*s).to_owned()).collect()),
            "{line:?}"
        );
    }
}

#[test]
fn an_unclosed_quote_and_a_control_character_are_refused() {
    assert_eq!(arguments(r#"--x="a b"#), Err(ValueError::QuoteNotClosed));
    assert_eq!(arguments(r#"""""#), Err(ValueError::QuoteNotClosed));
    for line in ["a\tb", "--stdio\u{0}", "\"a\u{7}\"", "x\u{85}"] {
        assert_eq!(
            arguments(line),
            Err(ValueError::ArgumentControl),
            "{line:?}"
        );
    }
}

/// Backslashes survive, so a Windows path is an argument like any other.
#[test]
fn arguments_round_trip_through_their_line() {
    let cases: [&[&str]; 9] = [
        &["--stdio"],
        &["a b"],
        &[""],
        &["a\"b"],
        &["--x=a b"],
        &[r"C:\tools\x"],
        &["ü", "--x="],
        &["many   spaces", "and\u{a0}a no-break space"],
        &[],
    ];
    for case in cases {
        let owned: Vec<String> = case.iter().map(|s| (*s).to_owned()).collect();
        let line = Arguments::to_line(&owned);
        assert_eq!(arguments(&line), Ok(owned.clone()), "{line:?}");
    }
    assert_eq!(Arguments::to_line(&["--x=a b".to_owned()]), r#""--x=a b""#);
    assert_eq!(quote("a\"b"), r#""a""b""#);
    assert_eq!(quote(""), r#""""#);
    assert_eq!(quote("--stdio"), "--stdio");
}

/// Completion replaces the argument being typed: where it starts, and its
/// text as the grammar reads it.
#[test]
fn the_last_argument_is_the_one_being_typed() {
    assert_eq!(Arguments::last(""), (0, String::new()));
    assert_eq!(Arguments::last("--stdio "), (8, String::new()));
    assert_eq!(Arguments::last("--stdio --ver"), (8, "--ver".to_owned()));
    assert_eq!(Arguments::last(r#"a "b c"#), (2, "b c".to_owned()));
    assert_eq!(Arguments::last(r#"--x="a b"#), (0, "--x=a b".to_owned()));
    assert_eq!(Arguments::last("ü  ä"), (4, "ä".to_owned()));
}
