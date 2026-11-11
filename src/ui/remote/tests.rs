//! Unit tests for marked and chosen styling on remote panel rows.

use super::row;
use crate::models::DirEntry;
use crate::ui::{palette, Cells};

#[test]
fn marked_unselected_remote_row_uses_marked_foreground() {
    let entry = DirEntry {
        name: "archive.tar".to_owned(),
        ..DirEntry::default()
    };
    let line = row(&entry, false, true, Cells::new(16));
    assert_eq!(line.spans[0].style.fg, Some(palette::MARKED));
    assert!(line.spans[0].content.starts_with("* archive.tar"));
}
