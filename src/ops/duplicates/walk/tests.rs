//! Purpose: the corners of the walk that a folder tree alone does not reach on
//! every system: a root that is not a folder, a link, an entry that vanished
//! after it was listed, and a path that is not a regular file.

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::{read_text, walk, Budget, Walker};
use crate::mask::Mask;
use crate::ops::duplicates::tests::tree;

fn budget() -> Budget {
    Budget {
        files: usize::MAX,
        bytes: u64::MAX,
        deadline: Instant::now() + Duration::from_secs(60),
    }
}

fn paths(files: &[(PathBuf, String)]) -> Vec<PathBuf> {
    files.iter().map(|(path, _)| path.clone()).collect()
}

#[test]
fn a_root_that_cannot_be_listed_reads_nothing_and_does_not_stop_the_walk() {
    let dir = tree(&[("lone.rs", "text")]);
    for root in [dir.path().join("lone.rs"), dir.path().join("missing")] {
        let found = walk(&root, &Mask::parse("*"), budget());
        assert!(found.files.is_empty(), "{}", root.display());
        assert_eq!(found.stopped, None, "{}", root.display());
    }
}

#[test]
fn an_entry_whose_kind_cannot_be_read_is_left_alone() {
    let dir = tree(&[("a.rs", "text")]);
    let path = dir.path().join("a.rs");
    let mask = Mask::parse("*");
    let mut walker = Walker {
        mask: &mask,
        budget: budget(),
        files: Vec::new(),
        bytes: 0,
        stopped: None,
    };
    walker.by_kind(&path, "a.rs", None);
    assert_eq!(walker.files.len(), 0, "nothing is read");
    assert_eq!(walker.stopped, None);
    let kind = fs::metadata(&path).expect("metadata").file_type();
    walker.by_kind(&path, "a.rs", Some(kind));
    assert_eq!(paths(&walker.files), [path]);
}

#[test]
fn a_linked_folder_is_not_entered() {
    let dir = tree(&[("real/inside.rs", "i")]);
    crate::test_support::make_dir_link(&dir.path().join("real"), &dir.path().join("link"))
        .expect("folder link");
    let found = walk(dir.path(), &Mask::parse("*"), budget());
    assert_eq!(
        paths(&found.files),
        [dir.path().join("real").join("inside.rs")],
        "the link is skipped, so nothing is read twice"
    );
}

#[test]
fn a_folder_is_not_source_text() {
    let dir = tree(&[("sub/inner.rs", "inner")]);
    assert_eq!(read_text(&dir.path().join("sub")), None);
    assert_eq!(
        read_text(&dir.path().join("sub").join("inner.rs")).as_deref(),
        Some("inner")
    );
}
