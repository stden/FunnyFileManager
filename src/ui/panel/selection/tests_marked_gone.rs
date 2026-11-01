//! Purpose: Marks naming entries the panel no longer lists add no selection
//! summary to the footer.

use super::*;
use crate::models::FileName;

#[test]
fn marks_on_entries_no_longer_listed_add_no_summary() {
    let root = tempfile::TempDir::new().expect("test folder");
    let mut panel = Panel::new(root.path().to_owned()).expect("test panel");
    panel.marked.insert(FileName::new("gone.txt"));
    assert_eq!(selection_summary(&panel, Cells::new(60)), None);
}
