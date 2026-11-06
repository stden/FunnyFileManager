//! Unit tests: which file a load reads, what it reads from it, and when a
//! save refuses to touch the file. What a load reads is in `tests/load.rs`,
//! the files it refuses to read in `tests/read_limits.rs`, the stored files a
//! save refuses to replace in `tests/refusals.rs`, what a save writes in
//! `tests/merge.rs`, an edit of the file's own mapping in `tests/edit.rs`, and
//! the files no writer may replace in `tests/safety.rs`.

use std::fs::{self, File};

use serde::{Deserialize, Serialize};
use serde_norway::{Mapping, Value};
use tempfile::TempDir;

use super::*;

mod edit;
mod load;
mod merge;
mod read_limits;
mod refusals;
mod safety;

/// A stand-in owner of one top-level key, `counter`.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Counter {
    counter: u32,
}

/// Where a fixture file goes in a scenario's temp dir.
type FixturePath = fn(&TempDir) -> PathBuf;

fn settings_path(temp: &TempDir) -> PathBuf {
    temp.path().join("ffm").join(FILE_NAME)
}

fn legacy_path(temp: &TempDir) -> PathBuf {
    temp.path().join("ffm").join(LEGACY_FILE_NAME)
}

fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("config dir");
    fs::write(path, text).expect("settings fixture");
}

fn stored(path: &Path) -> Stored {
    read(path).expect("readable").expect("a stored file")
}

/// Save `counter` as a writer that loaded nothing, so holds the defaults.
fn save(path: &Path, counter: u32) -> io::Result<()> {
    update(path, &Counter::default(), &Counter { counter })
}

#[test]
fn save_handles_parentless_paths_and_hard_lock_errors() {
    assert!(save(Path::new(""), 1).is_err());
    let err = save::take_lock_with(
        || Err(fs::TryLockError::Error(io::Error::other("lock fault"))),
        Path::new("settings.yaml"),
        Path::new("settings.yaml.lock"),
    )
    .unwrap_err();
    assert!(err.to_string().contains("settings.yaml.lock: lock fault"));
}

/// The lock of a path with no parent folder sits in the current directory.
/// The working directory is process-wide, so the check runs in a child.
#[cfg(unix)]
#[test]
fn a_path_without_a_parent_takes_its_lock_in_the_current_directory() {
    crate::test_support::filesystem_faults::run(|| {
        let root = TempDir::new().unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(root.path()).unwrap();

        let saved = save(Path::new(""), 1);

        std::env::set_current_dir(previous).unwrap();
        assert!(saved.is_err());
        assert!(root.path().join(LOCK_SUFFIX).is_file());
    });
}
