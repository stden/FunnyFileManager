//! Purpose: Unit tests for `Search` file-type errors, text-scan timeouts, and combined limit reporting.

use std::fs;
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::results::{report, Limit};
use super::Search;
use crate::models::{App, Panel};

#[test]
fn consider_ignores_unreadable_file_type_and_text_matches_marks_timeout() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("alpha.txt");
    fs::write(&file, "alpha").expect("write");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);

    let entry = fs::read_dir(temp.path())
        .expect("read_dir")
        .next()
        .expect("entry")
        .expect("ok entry");
    let expired = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    let mut search = Search::prepare(&app, "*", false, "alpha", expired);
    let mut pending = Vec::new();
    search.consider(temp.path(), &entry, None, 0, &mut pending);
    assert_eq!(search.rows.len(), 0);

    assert!(!search.text_matches(&file, None));
    assert_eq!(search.stopped, Some(Limit::Time));

    let future = Instant::now() + Duration::from_secs(10);
    let mut live = Search::prepare(&app, "*.txt", false, "alpha", future);
    search.consider(temp.path(), &entry, entry.file_type().ok(), 0, &mut pending);
    live.consider(temp.path(), &entry, entry.file_type().ok(), 0, &mut pending);
    assert_eq!(live.rows.len(), 1);

    report(&mut app, Some(Limit::Entries), 1);
    assert_eq!(
        app.status.to_string(),
        "Search stopped at the entry limit and skipped 1 folder that could not be read."
    );
}
