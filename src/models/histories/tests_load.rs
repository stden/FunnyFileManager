//! Purpose: A saved history loads its lines, and skips the empty ones.

use std::fs;

use super::*;
use crate::test_support::EnvVarGuard;

#[test]
fn a_saved_history_loads_its_non_empty_lines() {
    let root = tempfile::TempDir::new().expect("test folder");
    let _data = EnvVarGuard::set("XDG_DATA_HOME", root.path());
    let folder = crate::config::data_dir().expect("a data folder under the test folder");
    fs::create_dir_all(&folder).expect("the data folder");
    fs::write(
        folder.join(<Visited as HistoryFile>::FILE),
        "first\n\nsecond\n",
    )
    .expect("the history file");
    let history = PathHistory::<Visited>::load();
    assert_eq!(history.newest_first().count(), 2);
}

#[test]
fn a_saved_visited_history_loads_its_lines_too() {
    let root = tempfile::TempDir::new().expect("test folder");
    let _data = EnvVarGuard::set("XDG_DATA_HOME", root.path());
    let folder = crate::config::data_dir().expect("a data folder under the test folder");
    fs::create_dir_all(&folder).expect("the data folder");
    fs::write(
        folder.join(<Viewed as HistoryFile>::FILE),
        "first\nsecond\n",
    )
    .expect("the history file");
    let history = PathHistory::<Viewed>::load();
    assert_eq!(history.newest_first().count(), 2);
}
