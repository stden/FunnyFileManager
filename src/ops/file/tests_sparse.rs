//! Unit tests: a sparse file is copied without filling in its holes.
//!
//! A disk image or a database file is often mostly hole — `truncate -s 5G`
//! allocates nothing. A copy that writes every hole out as zeros turns a copy
//! that should cost nothing into one that fills the disk.

use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;

use super::tests_transfer_policies::{app_with, mark, settings};
use super::*;

/// Large enough that a filled-in hole cannot hide in block rounding, small
/// enough to copy in a blink.
const SPARSE_SIZE: u64 = 16 * 1024 * 1024;

/// Bytes the filesystem actually allocated to `path`.
fn allocated(path: &Path) -> u64 {
    fs::metadata(path).expect("metadata").blocks() * 512
}

/// A [`SPARSE_SIZE`] file holding `chunks` at their offsets and hole
/// everywhere else. `false` where the filesystem allocated the holes anyway,
/// which leaves nothing to test.
fn make_sparse(path: &Path, chunks: &[(u64, &[u8])]) -> bool {
    let mut file = fs::File::create(path).expect("test setup");
    file.set_len(SPARSE_SIZE).expect("test setup");
    for (offset, bytes) in chunks {
        file.seek(SeekFrom::Start(*offset)).expect("test setup");
        file.write_all(bytes).expect("test setup");
    }
    allocated(path) < SPARSE_SIZE / 2
}

fn assert_copied_sparse(source: &Path, copy: &Path) {
    assert_eq!(
        fs::read(copy).expect("the copy"),
        fs::read(source).expect("the source"),
        "the same bytes, holes read as zeros"
    );
    assert!(
        allocated(copy) < SPARSE_SIZE / 2,
        "the holes were written out: {} of {SPARSE_SIZE} bytes allocated",
        allocated(copy)
    );
}

#[test]
fn f5_of_a_sparse_file_keeps_its_holes() {
    let (mut app, from, to) = app_with(&[]);
    let source = from.path().join("disk.img");
    if !make_sparse(&source, &[(0, b"head"), (SPARSE_SIZE / 2, b"middle")]) {
        return;
    }
    mark(&mut app, "disk.img");

    copy_with(&mut app, &settings(to.path()));

    assert_copied_sparse(&source, &to.path().join("disk.img"));
}

/// A file that ends in a hole has nothing written for its tail, and the copy
/// must still come out at full length.
#[test]
fn a_sparse_file_ending_in_a_hole_keeps_its_length() {
    let (mut app, from, to) = app_with(&[]);
    let source = from.path().join("disk.img");
    if !make_sparse(&source, &[(0, b"head")]) {
        return;
    }
    mark(&mut app, "disk.img");

    copy_with(&mut app, &settings(to.path()));

    let copy = to.path().join("disk.img");
    assert_eq!(fs::metadata(&copy).expect("copied").len(), SPARSE_SIZE);
    assert_copied_sparse(&source, &copy);
}

#[test]
fn f5_of_a_folder_keeps_the_holes_of_the_files_inside() {
    let (mut app, from, to) = app_with(&[]);
    fs::create_dir(from.path().join("images")).expect("test setup");
    let source = from.path().join("images/disk.img");
    if !make_sparse(&source, &[(SPARSE_SIZE - 4, b"tail")]) {
        return;
    }
    mark(&mut app, "images");

    copy_with(&mut app, &settings(to.path()));

    assert_copied_sparse(&source, &to.path().join("images/disk.img"));
}
