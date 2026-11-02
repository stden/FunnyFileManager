//! Purpose: Pin every value the Language servers form accepts or refuses, one
//! row per case, and the argument grammar's round trip. Arguments are in
//! `tests/arguments.rs`, extensions in `tests/extensions.rs`.

use super::*;

mod arguments;
mod extensions;

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
