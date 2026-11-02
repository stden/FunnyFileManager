//! Purpose: A report is not a file, so the viewer has no file to point at.

use super::*;

#[test]
fn a_report_has_no_viewed_file() {
    let viewer = Viewer::report("Summary", vec!["done".to_owned()]);
    assert!(viewer.viewed_file().is_none());
}
