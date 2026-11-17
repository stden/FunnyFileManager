//! Purpose: What a tree weighs — the total, and each subdirectory's share of it.

use super::*;

#[test]
fn an_empty_directory_measures_zero() {
    let dir = TempDir::new().expect("temp dir");
    let stats = analyse(dir.path());

    assert_eq!(stats.total.bytes(), 0);
    assert_eq!(stats.files, 0);
    assert_eq!(stats.directories, 0);
    assert_eq!(stats.children.len(), 0);
    assert_eq!(stats.by_kind.len(), 0);
    assert!(stats.coverage.is_complete());
}

/// A subdirectory's size is everything beneath it, not what sits in it.
#[test]
fn a_subdirectory_carries_the_weight_of_its_whole_subtree() {
    let dir = tree(&[
        ("big/nested/deep/a.bin", 3000),
        ("big/b.bin", 1000),
        ("small/c.bin", 10),
    ]);
    let stats = analyse(dir.path());

    assert_eq!(child(&stats, "big").unwrap().size.bytes(), 4000);
    assert_eq!(child(&stats, "big").unwrap().files, 2);
    assert_eq!(child(&stats, "small").unwrap().size.bytes(), 10);
    assert_eq!(stats.total.bytes(), 4010);
    assert_eq!(stats.files, 3, "files anywhere in the tree are counted");
}

/// The point of the view: the biggest subdirectory is the first thing shown.
#[test]
fn subdirectories_are_ordered_by_size() {
    let dir = tree(&[("a/x", 10), ("b/x", 5000), ("c/x", 300)]);
    let names: Vec<String> = analyse(dir.path())
        .children
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["b", "c", "a"]);
}

#[test]
fn files_at_the_top_level_count_toward_the_total() {
    let dir = tree(&[("loose.bin", 700), ("sub/inner.bin", 300)]);
    let stats = analyse(dir.path());

    assert_eq!(stats.total.bytes(), 1000);
    assert_eq!(stats.files, 2);
    assert_eq!(stats.directories, 1);
}

/// `special/` read "5.0 G in 1 files, 0 folders": the socket and the device
/// beside the image went uncounted, though the panel counts them as files.
/// They are counted now, at the top and below it, without adding bytes.
#[cfg(unix)]
#[test]
fn sockets_and_fifos_count_as_files_without_bytes() {
    let dir = tree(&[("image.img", 900), ("sub/inner.bin", 100)]);
    let _top = std::os::unix::net::UnixListener::bind(dir.path().join("socket")).expect("socket");
    crate::test_support::make_fifo(&dir.path().join("sub/pipe")).expect("fifo");

    let stats = analyse(dir.path());

    assert_eq!(stats.files, 4);
    assert_eq!(stats.total.bytes(), 1000);
    assert_eq!(child(&stats, "sub").map(|sub| sub.files), Some(2));
    assert_eq!(stats.largest.len(), 2, "only files with data are listed");
}

#[test]
fn repeated_analyse_reuses_cached_directory_stats() {
    let dir = tree(&[("a.bin", 400), ("sub/b.bin", 600)]);
    let first = analyse(dir.path());
    let second = analyse(dir.path());
    assert_eq!(first, second);
    assert_eq!(second.total.bytes(), 1000);
}
