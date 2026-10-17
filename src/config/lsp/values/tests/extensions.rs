//! Purpose: Pin the extensions a server form accepts or refuses, and the line
//! that reads back what the file holds.

use super::*;

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
