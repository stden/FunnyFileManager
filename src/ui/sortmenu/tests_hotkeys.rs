//! Purpose: A menu label with nothing in it draws no hotkey spans.

use super::*;

#[test]
fn an_empty_label_draws_no_hotkey_spans() {
    assert_eq!(hotkey_spans("", Style::default(), true).len(), 0);
}
