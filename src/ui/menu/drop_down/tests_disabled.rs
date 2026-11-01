//! Purpose: A menu entry the program does not have yet is drawn greyed out, and
//! an available one keeps the menu's own colours.

use super::*;

#[test]
fn an_unavailable_entry_is_greyed_and_an_available_one_is_not() {
    let ground = Style::default();
    assert_eq!(
        entry_style(false, false, ground).fg,
        Some(palette::DISABLED_FG)
    );
    assert_eq!(entry_style(false, true, ground), ground);
}
