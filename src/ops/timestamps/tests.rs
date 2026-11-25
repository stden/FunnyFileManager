//! Purpose: Unit tests for partial timestamp updates (`modified` only and `accessed` only).

use std::fs;
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

use super::set_times;
use crate::models::RequestedTimes;

fn seed_file(dir: &TempDir, name: &str, time: SystemTime) -> std::path::PathBuf {
    let file = dir.path().join(name);
    fs::write(&file, "data").expect("write");
    set_times(
        &file,
        RequestedTimes {
            modified: Some(time),
            accessed: Some(time),
        },
    )
    .expect("seed baseline times");
    file
}

#[test]
fn set_times_updates_accessed_without_modifying_mtime() {
    let temp = TempDir::new().expect("tempdir");
    let initial = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let updated = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100_000_000);
    let file = seed_file(&temp, "access.txt", initial);

    set_times(
        &file,
        RequestedTimes {
            modified: None,
            accessed: Some(updated),
        },
    )
    .expect("set accessed only");
    let meta = fs::metadata(&file).expect("metadata");
    assert_eq!(meta.modified().expect("mtime"), initial);
    assert_eq!(meta.accessed().expect("atime"), updated);
}

#[test]
fn set_times_updates_modified_without_requiring_accessed() {
    let temp = TempDir::new().expect("tempdir");
    let initial = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let updated = SystemTime::UNIX_EPOCH + Duration::from_secs(1_100_000_000);
    let file = seed_file(&temp, "modify.txt", initial);

    set_times(
        &file,
        RequestedTimes {
            modified: Some(updated),
            accessed: None,
        },
    )
    .expect("set modified only");
    let meta = fs::metadata(&file).expect("metadata");
    assert_eq!(meta.modified().expect("mtime"), updated);
}
