//! Purpose: A rename opens on the current name and an empty link target, and
//! stepping the link drop-down keeps the cursor on it.

use super::NamePrompt;
use crate::models::{FileName, LinkType, NameField};

#[test]
fn target_line_of_a_rename_starts_empty() {
    let prompt = NamePrompt::rename(FileName::new("a.txt"));
    assert_eq!(prompt.name(), "a.txt");
    assert_eq!(prompt.target_line().text(), "");
}

#[test]
fn cycling_link_types_keeps_focus_on_the_available_dropdown() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Link);
    for _ in 0..6 {
        prompt.cycle(true);
        assert_eq!(prompt.field(), NameField::Link);
        assert!(prompt.allows(prompt.field()));
    }
    assert_eq!(LinkType::Hard.label(), "hard link");
}
