//! Purpose: A rename opens on the current name and an empty link target.

use super::NamePrompt;
use crate::models::FileName;

#[test]
fn target_line_of_a_rename_starts_empty() {
    let prompt = NamePrompt::rename(FileName::new("a.txt"));
    assert_eq!(prompt.name(), "a.txt");
    assert_eq!(prompt.target_line().text(), "");
}
