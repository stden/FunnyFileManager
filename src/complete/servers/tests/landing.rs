//! Purpose: Pin the typed token as a candidate spells it, and how an accepted
//! suggestion lands in each field.

use super::*;

#[test]
fn the_typed_token_is_spelled_like_a_candidate() {
    let line = |text: &str| LineEditor::new(text);
    assert_eq!(typed(CompletedField::Extensions, &line("rs, py")), ".py");
    assert_eq!(typed(CompletedField::Extensions, &line("rs ")), "");
    assert_eq!(typed(CompletedField::Arguments, &line(r#"a "b c"#)), "b c");
    assert_eq!(typed(CompletedField::Command, &line("  pylsp")), "pylsp");
}

#[test]
fn an_accepted_command_replaces_the_token_without_a_space() {
    let mut line = line_at("py", "");
    accept(CompletedField::Command, &mut line, "pylsp");
    assert_eq!(line.text(), "pylsp");
    assert_eq!(line.before_cursor(), "pylsp");
}

#[test]
fn an_accepted_argument_is_quoted_when_it_must_be_and_spaced() {
    let mut line = line_at("--stdio --lo", "");
    accept(CompletedField::Arguments, &mut line, "--log=a b");
    assert_eq!(line.text(), r#"--stdio "--log=a b" "#);

    let mut line = line_at("st", " --verbose");
    accept(CompletedField::Arguments, &mut line, "start");
    assert_eq!(line.text(), "start --verbose");
    assert_eq!(line.after_cursor(), " --verbose");
}

#[test]
fn an_accepted_extension_replaces_the_word_with_its_dotted_form() {
    let mut line = line_at("rs, p", "");
    accept(CompletedField::Extensions, &mut line, ".pyi");
    assert_eq!(line.text(), "rs, .pyi ");

    let mut line = line_at(".p", " .rs");
    accept(CompletedField::Extensions, &mut line, ".py");
    assert_eq!(line.text(), ".py .rs");
}
