//! Purpose: Unit tests for `NamePrompt::toggle` on checkbox and non-checkbox controls.

use super::{NameField, NamePrompt};
use crate::models::FileName;

#[test]
fn toggle_only_flips_multiple_when_checkbox_is_focused_and_allowed() {
    let mut folder = NamePrompt::make_folder();
    assert_eq!(folder.field(), NameField::Name);
    folder.toggle();
    assert!(!folder.multiple());

    folder.focus(NameField::Multiple);
    folder.toggle();
    assert!(folder.multiple());

    let mut rename = NamePrompt::rename(FileName::new("a.txt"));
    rename.toggle();
    assert!(!rename.multiple());
}
