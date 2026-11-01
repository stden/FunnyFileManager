//! Purpose: Temporary-file cleanup is confined to one injected root and a stable preview.
use super::*;
use crate::models::temp_cleanup::{TempError, TempSource};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};
fn source(root: &Path) -> TempSource {
    TempSource::Directory {
        path: root.to_owned(),
        home: None,
    }
}
fn old_file(path: &Path, bytes: &[u8], now: SystemTime) {
    fs::write(path, bytes).unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(now - Duration::from_hours(192))
        .unwrap();
}
#[test]
fn confirmed_plan_removes_old_files_and_keeps_root_and_fresh_files() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temp");
    fs::create_dir(&root).unwrap();
    let now = SystemTime::now();
    old_file(&root.join("old.bin"), b"old", now);
    fs::write(root.join("fresh.bin"), b"fresh").unwrap();
    let plan = scan(&source(&root), now).unwrap();
    assert_eq!(plan.files.len(), 1);
    assert_eq!(plan.bytes.bytes(), 3);
    let outcome = remove(&plan);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.freed.bytes(), 3);
    assert!(root.is_dir());
    assert_eq!(fs::read(root.join("fresh.bin")).unwrap(), b"fresh");
    assert!(!root.join("old.bin").exists());
}
#[test]
fn changed_and_new_files_after_preview_are_preserved() {
    let temp = tempfile::TempDir::new().unwrap();
    let now = SystemTime::now();
    old_file(&temp.path().join("changed"), b"old", now);
    let plan = scan(&source(temp.path()), now).unwrap();
    fs::write(temp.path().join("changed"), b"new content").unwrap();
    old_file(&temp.path().join("new-old-looking-file"), b"new", now);
    let outcome = remove(&plan);
    assert_eq!(outcome.removed, 0);
    assert_eq!(
        fs::read(temp.path().join("changed")).unwrap(),
        b"new content"
    );
    assert!(temp.path().join("new-old-looking-file").exists());
}

mod safety;

mod policy;
