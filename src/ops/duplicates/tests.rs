//! Purpose: the Find duplicates walk: what it reads, in what order, and where it stops.

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use tempfile::TempDir;

use super::walk::{walk, Budget, Limit, Walk, MAX_FILE_BYTES};
use crate::mask::Mask;

/// A folder holding each relative path with its text.
pub(super) fn tree(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    for (relative, text) in files {
        let path = dir.path().join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, text).expect("file");
    }
    dir
}

fn budget(files: usize) -> Budget {
    Budget {
        files,
        bytes: u64::MAX,
        deadline: Instant::now() + Duration::from_secs(60),
    }
}

fn read(root: &Path, pattern: &str) -> Walk {
    walk(root, &Mask::parse(pattern), budget(usize::MAX))
}

fn paths(found: &Walk) -> Vec<std::path::PathBuf> {
    found.files.iter().map(|(path, _)| path.clone()).collect()
}

#[test]
fn reads_matching_text_files_in_path_order() {
    let dir = tree(&[
        ("b.rs", "b"),
        ("a.rs", "a"),
        ("notes.txt", "n"),
        ("sub/c.rs", "c"),
    ]);
    let found = read(dir.path(), "*.rs");
    assert_eq!(
        paths(&found),
        [
            dir.path().join("a.rs"),
            dir.path().join("b.rs"),
            dir.path().join("sub").join("c.rs"),
        ]
    );
    assert_eq!(found.stopped, None);
    assert_eq!(
        found.files.first().map(|(_, text)| text.as_str()),
        Some("a")
    );
}

#[test]
fn skips_hidden_and_build_folders_and_hidden_files() {
    let dir = tree(&[
        ("keep.rs", "k"),
        ("src/main.rs", "m"),
        (".hidden/x.rs", "x"),
        ("src/.cache.rs", "c"),
        ("target/y.rs", "y"),
        ("node_modules/z.rs", "z"),
        ("venv/w.rs", "w"),
        ("__pycache__/v.rs", "v"),
    ]);
    let found = read(dir.path(), "*");
    assert_eq!(
        paths(&found),
        [
            dir.path().join("keep.rs"),
            dir.path().join("src").join("main.rs")
        ]
    );
}

#[cfg(unix)]
#[test]
fn never_follows_a_symbolic_link() {
    let dir = tree(&[("real/inside.rs", "i")]);
    std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link"))
        .expect("folder link");
    std::os::unix::fs::symlink(
        dir.path().join("real").join("inside.rs"),
        dir.path().join("alias.rs"),
    )
    .expect("file link");
    let found = read(dir.path(), "*");
    assert_eq!(
        paths(&found),
        [dir.path().join("real").join("inside.rs")],
        "neither link is read, so nothing is read twice"
    );
}

#[test]
fn skips_binary_non_utf8_and_oversized_files() {
    let dir = tree(&[("ok.rs", "fine")]);
    fs::write(dir.path().join("binary.rs"), b"text\0more").expect("binary");
    fs::write(dir.path().join("latin.rs"), [0xff_u8, 0xfe, b'a']).expect("latin");
    fs::write(
        dir.path().join("big.rs"),
        vec![b'a'; to_usize(MAX_FILE_BYTES) + 1],
    )
    .expect("big");
    fs::write(
        dir.path().join("edge.rs"),
        vec![b'a'; to_usize(MAX_FILE_BYTES)],
    )
    .expect("edge");
    let found = read(dir.path(), "*");
    assert_eq!(
        paths(&found),
        [dir.path().join("edge.rs"), dir.path().join("ok.rs")],
        "a file of exactly the limit is read; one byte more is not"
    );
}

fn to_usize(bytes: u64) -> usize {
    usize::try_from(bytes).expect("test size fits in memory")
}

#[test]
fn a_walk_that_fits_its_file_budget_is_not_stopped() {
    let dir = tree(&[("a.rs", "a"), ("b.rs", "b")]);
    let found = walk(dir.path(), &Mask::parse("*"), budget(2));
    assert_eq!(found.files.len(), 2);
    assert_eq!(found.stopped, None);
}

#[test]
fn stops_at_the_file_budget_and_says_so() {
    let dir = tree(&[("a.rs", "a"), ("b.rs", "b"), ("c.rs", "c")]);
    let found = walk(dir.path(), &Mask::parse("*"), budget(2));
    assert_eq!(found.files.len(), 2);
    assert_eq!(found.stopped, Some(Limit::Files));
}

#[test]
fn stops_at_the_byte_budget_and_says_so() {
    let dir = tree(&[
        ("a.rs", "0123456789"),
        ("b.rs", "0123456789"),
        ("c.rs", "0123456789"),
    ]);
    let small = Budget {
        files: usize::MAX,
        bytes: 15,
        deadline: Instant::now() + Duration::from_secs(60),
    };
    let found = walk(dir.path(), &Mask::parse("*"), small);
    assert_eq!(
        found.files.len(),
        2,
        "the second file starts under 15 bytes"
    );
    assert_eq!(found.stopped, Some(Limit::Files));
}

#[test]
fn stops_at_the_time_budget_and_says_so() {
    let dir = tree(&[("a.rs", "a")]);
    let expired = Budget {
        files: usize::MAX,
        bytes: u64::MAX,
        deadline: Instant::now(),
    };
    let found = walk(dir.path(), &Mask::parse("*"), expired);
    assert_eq!(found.files.len(), 0, "nothing is read");
    assert_eq!(found.stopped, Some(Limit::Time));
}
