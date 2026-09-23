//! Unit tests: mapping Colorer's region vocabulary onto the editor's palette.
//!
//! The mapping must work without a scheme database present, so it is tested on
//! its own. Tests that need a real catalog step aside when there is none — a
//! machine without Colorer installed is an ordinary machine, not a failure.

use super::*;

/// `def:` regions are the common vocabulary every scheme inherits.
#[test]
fn the_def_regions_map_onto_the_palette() {
    assert_eq!(region_token("def:Keyword"), Token::Keyword);
    assert_eq!(region_token("def:String"), Token::Str);
    assert_eq!(region_token("def:Comment"), Token::Comment);
    assert_eq!(region_token("def:Number"), Token::Number);
    assert_eq!(region_token("def:NumberDec"), Token::Number);
    assert_eq!(region_token("def:Symbol"), Token::Punctuation);
    assert_eq!(region_token("def:Text"), Token::Normal);
}

/// The point of matching on the suffix: a language this code has never heard
/// of still lands on the right colour, because Colorer names its regions after
/// the `def:` ones.
#[test]
fn a_language_prefix_does_not_hide_the_region() {
    assert_eq!(region_token("python:pyKeyword"), Token::Keyword);
    assert_eq!(region_token("python:pyStringEdge"), Token::Str);
    assert_eq!(region_token("python:pyNumberDec"), Token::Number);
    assert_eq!(region_token("rust:rustComment"), Token::Comment);
    // A language nobody wrote a table for still resolves.
    assert_eq!(region_token("erlang:erlKeyword"), Token::Keyword);
}

/// The order of the rules matters: a compound name must be read by its most
/// specific part, not by whichever needle happens to appear first.
#[test]
fn compound_names_resolve_to_the_more_specific_class() {
    assert_eq!(region_token("def:CommentContent"), Token::Comment);
    assert_eq!(region_token("def:StringEdge"), Token::Str);
    assert_eq!(region_token("def:CommentDoc"), Token::Comment);
}

#[test]
fn an_unknown_region_is_plain_rather_than_a_guess() {
    assert_eq!(region_token("def:Something"), Token::Normal);
    assert_eq!(region_token(""), Token::Normal);
    assert_eq!(region_token("no-colon-at-all"), Token::Normal);
}

/// Everything below needs a real scheme database, so it steps aside when there
/// is none.
fn catalog() -> Option<&'static Colorer> {
    Colorer::shared()
}

#[test]
fn a_real_catalog_recognises_common_languages() {
    let Some(colorer) = catalog() else { return };
    assert_eq!(
        colorer.file_type(Path::new("main.rs")).as_deref(),
        Some("rust")
    );
    assert_eq!(
        colorer.file_type(Path::new("app.py")).as_deref(),
        Some("python")
    );
    // Something with no scheme answers "no" rather than Colorer's `default`.
    assert_eq!(
        colorer.file_type(Path::new("x.zzzz")),
        None,
        "an unrecognised extension must not report Colorer's `default` type"
    );
}

#[test]
fn a_real_catalog_finds_the_regions_in_a_rust_file() {
    let Some(colorer) = catalog() else { return };
    let lines: Vec<String> = ["// note", "fn main() {", "    let x = 42;", "}"]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

    let spans = colorer.parse(Path::new("main.rs"), &lines);
    assert_eq!(spans.len(), lines.len(), "one entry per line");

    let tokens: Vec<Token> = spans.iter().flatten().map(|span| span.token).collect();
    assert!(
        tokens.contains(&Token::Keyword),
        "`fn` and `let`: {spans:?}"
    );
    assert!(tokens.contains(&Token::Number), "`42`: {spans:?}");

    // Every span must lie inside its line, or the editor would slice past the
    // end of a string when it paints.
    for (index, row) in spans.iter().enumerate() {
        let width = lines[index].chars().count();
        for span in row {
            assert!(span.start < span.end, "empty span on line {index}");
            assert!(
                span.end <= width,
                "span {span:?} runs past line {index} ({width} chars)"
            );
        }
    }
}

/// A file Colorer has no scheme for must come back empty rather than wrong.
#[test]
fn an_unknown_file_type_yields_no_regions() {
    let Some(colorer) = catalog() else { return };
    let lines = vec!["some text".to_owned()];
    let spans = colorer.parse(Path::new("notes.zzzz"), &lines);
    assert!(spans.iter().all(Vec::is_empty), "{spans:?}");
}

/// The reason for all of this: Colorer knows languages the built-in tables do
/// not, so a file the tokenizer would leave plain comes out coloured.
#[test]
fn colorer_covers_languages_the_built_in_tables_do_not() {
    let Some(_) = catalog() else { return };
    let path = Path::new("main.lua");
    // The built-in table has no entry for Lua.
    assert!(
        crate::highlight::syntax_for(path).is_none(),
        "precondition: the tokenizer does not know Lua"
    );

    let lines: Vec<String> = ["-- a comment", "local x = 42", "function f() end"]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

    let mut highlighter = crate::highlight::Highlighter::for_file(path, &lines);
    let coloured: Vec<Token> = lines
        .iter()
        .flat_map(|line| highlighter.line(line))
        .map(|(_, token)| token)
        .collect();

    assert!(
        coloured.iter().any(|token| *token != Token::Normal),
        "Colorer must colour a file the tokenizer leaves plain: {coloured:?}"
    );
}

/// …and the runs must still reconstruct the line exactly, or the editor would
/// draw something the file does not contain.
#[test]
fn the_painted_runs_reproduce_the_line() {
    let Some(_) = catalog() else { return };
    let lines = vec!["let mut счёт = \"значение\"; // note".to_owned()];
    let mut highlighter = crate::highlight::Highlighter::for_file(Path::new("main.rs"), &lines);
    let rejoined: String = highlighter
        .line(&lines[0])
        .iter()
        .map(|(text, _)| *text)
        .collect();
    assert_eq!(rejoined, lines[0], "runs must cover the line exactly");
}
