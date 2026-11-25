//! Purpose: Unit tests for `markup::push` skipping empty segments.

use super::*;

#[test]
fn push_skips_empty_runs_and_empty_tag_names() {
    let mut runs = Vec::new();
    push(&mut runs, "", Token::Normal);
    assert_eq!(runs, Vec::<(&str, Token)>::new());

    let parsed = State::default().line("</>");
    assert_eq!(
        parsed,
        vec![("</", Token::Punctuation), (">", Token::Punctuation)]
    );
}
