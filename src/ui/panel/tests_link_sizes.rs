//! Purpose: What the Size column says for links: a file link's target size,
//! and a word for a folder link and a dangling one, never the link's length.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;

use tempfile::TempDir;

use super::size_field;
use crate::models::{DirEntry, Panel};

fn listed<'a>(panel: &'a Panel, name: &str) -> &'a DirEntry {
    panel
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .expect("listed")
}

/// `to-readme` showed 17, the length of the path it holds, beside a
/// 37-byte target; a link to a folder read "Folder" like the folder itself.
#[test]
fn a_link_shows_what_it_points_at_not_its_own_length() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("readme.txt"), "x".repeat(37)).expect("target");
    fs::create_dir(dir.path().join("docs")).expect("folder");
    symlink("../../../somewhere/readme.txt", dir.path().join("dangling")).expect("link");
    symlink(dir.path().join("readme.txt"), dir.path().join("to-readme")).expect("link");
    symlink(dir.path().join("docs"), dir.path().join("to-docs")).expect("link");

    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");

    assert_eq!(size_field(listed(&panel, "to-readme")), "37");
    assert_eq!(size_field(listed(&panel, "to-docs")), "Link");
    assert_eq!(size_field(listed(&panel, "dangling")), "Broken");
    assert_eq!(size_field(listed(&panel, "docs")), "Folder");
    assert_eq!(size_field(listed(&panel, "readme.txt")), "37");
}
