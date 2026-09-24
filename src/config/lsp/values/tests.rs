//! Purpose: Pin every value the Language servers form accepts or refuses, one
//! row per case, and the argument grammar's round trip.

use super::*;

/// `enabled` is a boolean in `settings.yaml`, and in the legacy JSON file
/// FFM still reads; a word YAML 1.1 took for one (`on`) is a string.
#[test]
fn enablement_is_a_boolean_both_ways() {
    assert_eq!(
        serde_norway::to_string(&Enablement::Enabled).unwrap(),
        "true\n"
    );
    assert_eq!(
        serde_norway::to_string(&Enablement::Disabled).unwrap(),
        "false\n"
    );
    for (text, enablement) in [
        ("true", Enablement::Enabled),
        ("false", Enablement::Disabled),
    ] {
        assert_eq!(
            serde_norway::from_str::<Enablement>(text).unwrap(),
            enablement
        );
        assert_eq!(
            serde_json::from_str::<Enablement>(text).unwrap(),
            enablement
        );
    }
    assert!(serde_norway::from_str::<Enablement>("on").is_err());
    assert!(serde_json::from_str::<Enablement>("\"on\"").is_err());
    assert_eq!(Enablement::Enabled.toggled(), Enablement::Disabled);
    assert_eq!(Enablement::Disabled.toggled(), Enablement::Enabled);
}

#[test]
fn a_new_language_is_an_lsp_language_id() {
    let cases: [(&str, Result<&str, ValueError>); 9] = [
        ("python", Ok("python")),
        ("  c++ ", Ok("c++")),
        ("objective-c", Ok("objective-c")),
        ("x.y_z", Ok("x.y_z")),
        ("", Err(ValueError::LanguageEmpty)),
        ("   ", Err(ValueError::LanguageEmpty)),
        ("Python", Err(ValueError::LanguageCharacters)),
        ("-rust", Err(ValueError::LanguageCharacters)),
        ("my lang", Err(ValueError::LanguageCharacters)),
    ];
    for (text, expected) in cases {
        let parsed = LanguageId::parse(text);
        assert_eq!(
            parsed.as_ref().map(LanguageId::as_str),
            expected.as_ref().copied(),
            "{text:?}"
        );
    }
    assert_eq!(
        LanguageId::parse(&"a".repeat(64)).map(|id| id.as_str().len()),
        Ok(64)
    );
    assert_eq!(
        LanguageId::parse(&"a".repeat(65)),
        Err(ValueError::LanguageCharacters)
    );
}

/// A key the file already holds is kept exactly, whatever it looks like:
/// renaming is not offered, so it is never rewritten.
#[test]
fn a_key_from_the_file_is_kept_verbatim() {
    let key = LanguageId::from_key("My Language");
    assert_eq!(key.as_str(), "My Language");
    assert_eq!(key.to_string(), "My Language");
}

#[test]
fn a_command_is_trimmed_and_refuses_what_no_shell_would_fix() {
    let cases: [(&str, Result<&str, ValueError>); 7] = [
        ("pylsp", Ok("pylsp")),
        ("  /opt/bin/pylsp  ", Ok("/opt/bin/pylsp")),
        (r"C:\tools\server.exe", Ok(r"C:\tools\server.exe")),
        ("", Err(ValueError::CommandEmpty)),
        ("   ", Err(ValueError::CommandEmpty)),
        ("py\nlsp", Err(ValueError::CommandControl)),
        ("~/bin/pylsp", Err(ValueError::CommandTilde)),
    ];
    for (text, expected) in cases {
        let parsed = ServerCommand::parse(text).map(String::from);
        assert_eq!(parsed, expected.map(str::to_owned), "{text:?}");
    }
}

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

#[test]
fn an_extension_drops_one_dot_and_must_be_the_last_dot_suffix() {
    let cases: [(&str, Result<&str, ValueError>); 11] = [
        ("rs", Ok("rs")),
        (".rs", Ok("rs")),
        (".", Err(ValueError::ExtensionEmpty)),
        ("", Err(ValueError::ExtensionEmpty)),
        ("..rs", Err(ValueError::ExtensionDot(".rs".to_owned()))),
        ("d.ts", Err(ValueError::ExtensionDot("d.ts".to_owned()))),
        ("a/b", Err(ValueError::ExtensionSeparator)),
        (r"a\b", Err(ValueError::ExtensionSeparator)),
        // The form's line splits on these, so a stored one is reported.
        ("a,b", Err(ValueError::ExtensionSeparator)),
        ("a b", Err(ValueError::ExtensionSeparator)),
        ("r\u{1b}s", Err(ValueError::ExtensionControl)),
    ];
    for (text, expected) in cases {
        let parsed = Extension::parse(text).map(|extension| extension.0);
        assert_eq!(parsed, expected.map(str::to_owned), "{text:?}");
    }
}

fn extensions(line: &str) -> Result<Vec<String>, ValueError> {
    Extensions::parse_line(line).map(Vec::from)
}

#[test]
fn extensions_split_on_whitespace_and_commas_and_refuse_a_repeat() {
    assert_eq!(extensions(".py .pyi"), Ok(vec!["py".into(), "pyi".into()]));
    assert_eq!(
        extensions("py, pyi,  pyw"),
        Ok(vec!["py".into(), "pyi".into(), "pyw".into()])
    );
    assert_eq!(extensions("   "), Ok(Vec::new()));
    assert_eq!(extensions(".,"), Err(ValueError::ExtensionEmpty));
    assert_eq!(
        extensions("rs RS"),
        Err(ValueError::ExtensionTwice("rs".to_owned()))
    );
    assert_eq!(
        extensions(".ts d.ts"),
        Err(ValueError::ExtensionDot("d.ts".to_owned()))
    );
    assert_eq!(
        Extensions::to_line(&["py".to_owned(), "pyi".to_owned()]),
        ".py .pyi"
    );
}

fn assert_sentences(cases: &[(ValueError, &str)]) {
    for (error, message) in cases {
        assert_eq!(error.to_string(), *message);
    }
}

#[test]
fn command_and_argument_refusals_are_the_sentences_the_form_shows() {
    assert_sentences(&[
        (ValueError::CommandEmpty, "Enter a command."),
        (
            ValueError::CommandControl,
            "The command cannot contain control characters.",
        ),
        (
            ValueError::CommandTilde,
            "No shell runs the server, so ~ is not expanded; write the full path.",
        ),
        (ValueError::QuoteNotClosed, "A quote is not closed."),
        (
            ValueError::ArgumentControl,
            "An argument cannot contain control characters.",
        ),
    ]);
}

#[test]
fn extension_refusals_are_the_sentences_the_form_shows() {
    assert_sentences(&[
        (ValueError::ExtensionEmpty, "An extension cannot be empty."),
        (
            ValueError::ExtensionDot("d.ts".to_owned()),
            ".d.ts can never match: FFM compares the text after the last dot.",
        ),
        (
            ValueError::ExtensionSeparator,
            "Extensions cannot contain spaces, commas, / or \\.",
        ),
        (
            ValueError::ExtensionControl,
            "An extension cannot contain control characters.",
        ),
        (
            ValueError::ExtensionTwice("rs".to_owned()),
            ".rs is listed twice.",
        ),
    ]);
}

#[test]
fn language_refusals_are_the_sentences_the_form_shows() {
    assert_sentences(&[
        (
            ValueError::LanguageCharacters,
            "Use a-z, 0-9 and - _ . + (LSP languageId).",
        ),
        (
            ValueError::LanguageEmpty,
            "Enter the language: its LSP languageId.",
        ),
        (
            ValueError::LanguageTaken("python".to_owned()),
            "python already exists; edit it instead.",
        ),
    ]);
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

/// The form shows each stored extension with exactly one dot, so OK reads
/// back what the file holds: one written with its dot becomes the bare word.
#[test]
fn the_extensions_line_reads_back_what_the_file_holds() {
    let stored = ["rs".to_owned(), ".py".to_owned()];
    let line = Extensions::to_line(&stored);
    assert_eq!(line, ".rs .py");
    assert_eq!(extensions(&line), Ok(vec!["rs".into(), "py".into()]));
}

/// What splits the line is what an extension cannot hold, multibyte spaces
/// included.
#[test]
fn whitespace_and_commas_separate_extensions() {
    for separator in [' ', ',', '\u{a0}', '\u{3000}'] {
        assert!(Extensions::is_separator(separator), "{separator:?}");
        let word = format!("a{separator}b");
        assert_eq!(
            Extension::parse(&word).map(|extension| extension.0),
            Err(ValueError::ExtensionSeparator)
        );
    }
    assert!(!Extensions::is_separator('.'));
}
