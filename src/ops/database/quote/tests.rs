//! Purpose: Pin identifier quoting: double quotes around the name, with each inner double quote doubled.

use super::quote_identifier;

#[test]
fn a_plain_name_is_double_quoted() {
    assert_eq!(quote_identifier("users"), "\"users\"");
}

#[test]
fn spaces_and_dots_stay_inside_one_identifier() {
    assert_eq!(quote_identifier("my table.v1"), "\"my table.v1\"");
}

#[test]
fn inner_double_quotes_are_doubled() {
    assert_eq!(quote_identifier("say \"hi\""), "\"say \"\"hi\"\"\"");
}

#[test]
fn an_empty_name_is_still_two_quotes() {
    assert_eq!(quote_identifier(""), "\"\"");
}
