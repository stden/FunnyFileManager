//! Purpose: Unit tests for `regions::push_group` bounds and `None` handling.

use super::*;

#[test]
fn push_group_skips_missing_token_or_out_of_bounds_slice() {
    let mut runs = Vec::new();
    push_group(&mut runs, "abc", 0..1, None);
    push_group(&mut runs, "abc", 0..99, Some(&Token::Keyword));
    assert_eq!(runs, Vec::<(&str, Token)>::new());

    push_group(&mut runs, "abc", 0..3, Some(&Token::Keyword));
    assert_eq!(runs, vec![("abc", Token::Keyword)]);
}
