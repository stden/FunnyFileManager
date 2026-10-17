use crate::highlight::{Highlighter, Token};
use std::path::Path;

fn runs(line: &str) -> Vec<(&str, Token)> {
    let result = Highlighter::new(Path::new(".env")).line(line);
    assert_eq!(
        result.iter().map(|(text, _)| *text).collect::<String>(),
        line
    );
    result
}

#[test]
fn assignments_distinguish_export_names_values_and_comments() {
    let result = runs("  export SERVICE_URL = https://localhost:8443/api # local server");
    for expected in [
        ("export", Token::Keyword),
        ("SERVICE_URL", Token::Type),
        ("=", Token::Punctuation),
        ("https://localhost:8443/api", Token::Str),
        ("# local server", Token::Comment),
    ] {
        assert!(result.contains(&expected), "{expected:?}: {result:?}");
    }
    assert!(runs("exported=on").contains(&("exported", Token::Type)));
    assert_eq!(runs("# comment"), [("# comment", Token::Comment)]);
}

#[test]
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "the `${HOST}` reference is dotenv syntax, not a Rust formatting argument"
)]
fn quoted_hashes_and_variable_references_keep_their_context() {
    let result = runs(r#"URL="${HOST}:$PORT/#fragment" # trailing"#);
    assert!(result.contains(&("${HOST}", Token::Type)));
    assert!(result.contains(&("$PORT", Token::Type)));
    assert!(result.contains(&("/#fragment\"", Token::Str)));
    assert!(result.contains(&("# trailing", Token::Comment)));

    let literal = runs("PASSWORD='${VALUE} # literal'");
    assert!(literal.contains(&("'${VALUE} # literal'", Token::Str)));
    assert!(runs("URL=${HOST:-localhost}").contains(&("${HOST:-localhost}", Token::Type)));
}

#[test]
fn multiline_quotes_resume_and_do_not_swallow_the_next_assignment() {
    for quote in ['\'', '"'] {
        let mut highlighter = Highlighter::new(Path::new(".env.local"));
        let opening = format!("MESSAGE={quote}first");
        highlighter.line(&opening);
        let middle = "# всё ещё строка = value";
        assert_eq!(highlighter.line(middle), [(middle, Token::Str)]);
        let ending = format!("last{quote} # comment");
        let result = highlighter.line(&ending);
        assert!(result.contains(&("# comment", Token::Comment)));
        assert_eq!(
            result.iter().map(|(text, _)| *text).collect::<String>(),
            ending
        );
        assert!(highlighter
            .line("NEXT=yes")
            .contains(&("NEXT", Token::Type)));
    }
}

#[test]
fn quotes_inside_bare_values_do_not_open_multiline_strings() {
    let mut highlighter = Highlighter::new(Path::new(".env"));
    assert!(highlighter
        .line("MESSAGE=don't stop")
        .contains(&("don't", Token::Str)));
    assert!(highlighter
        .line("NEXT=value # comment")
        .contains(&("# comment", Token::Comment)));
    assert!(runs("MESSAGE =   'quoted # text'").contains(&("'quoted # text'", Token::Str)));
}

#[test]
fn escapes_unicode_and_incomplete_input_preserve_every_character() {
    for line in [
        "",
        "  ",
        "EMPTY=",
        "export INCOMPLETE",
        "INVALID-NAME=value",
        "1INVALID=value",
        "NAME=значение 🚀",
        r#"MESSAGE="say \"hello\" # still quoted" # comment"#,
        r"LITERAL=\$NAME\#value",
        "REFERENCE=${}",
        "REFERENCE=${UNFINISHED",
    ] {
        drop(runs(line));
    }
    let escaped = runs(r"LITERAL=\$NAME\#value");
    assert!(escaped.contains(&(r"\$NAME\#value", Token::Str)));
    assert!(runs("INVALID-NAME=value")
        .iter()
        .all(|(_, token)| *token == Token::Normal));
}
