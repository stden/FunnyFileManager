//! Unit tests: what the panel reports about symlinks.
//!
//! `fs::DirEntry::metadata` is an `lstat`, so the listing used to call a
//! symlinked directory a plain file of ~35 bytes. `Panel::enter` then refused to
//! descend (it checks `is_dir`) while F5/F6/F8 resolved the link — the entry was
//! unreachable by keyboard yet transferred as a whole directory tree.

#![cfg(unix)]

use crate::models::FileSize;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;

use super::{DirEntry, Panel};

use tempfile::TempDir;

fn entry<'a>(panel: &'a Panel, name: &str) -> &'a DirEntry {
    panel
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .expect("entry must be listed")
}

#[test]
fn a_directory_symlink_is_listed_as_an_enterable_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let real = temp_dir.path().join("real");
    fs::create_dir(&real).expect("test setup");
    symlink(&real, temp_dir.path().join("alias")).expect("test setup");

    let panel = Panel::new(temp_dir.path().to_path_buf())?;
    let alias = entry(&panel, "alias");

    assert!(
        alias.kind.is_dir(),
        "a link to a directory must behave like a directory — every file \
         operation already resolves it"
    );
    assert!(
        alias.kind.is_symlink(),
        "it is still a link, and must say so"
    );
    Ok(())
}

#[test]
fn enter_follows_a_directory_symlink() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let real = temp_dir.path().join("real");
    fs::create_dir(&real).expect("test setup");
    fs::write(real.join("inside.txt"), b"payload").expect("test setup");
    symlink(&real, temp_dir.path().join("alias")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "alias")
        .expect("test setup");
    panel.select(Some(index));

    panel.enter()?;

    assert_eq!(panel.path, temp_dir.path().join("alias"));
    assert!(
        panel.entries.iter().any(|entry| entry.name == "inside.txt"),
        "Enter on a symlinked directory must show the target's contents"
    );
    Ok(())
}

#[test]
fn a_file_symlink_stays_a_file_and_is_flagged_as_a_link() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::write(temp_dir.path().join("real.txt"), b"payload").expect("test setup");
    symlink("real.txt", temp_dir.path().join("alias.txt")).expect("test setup");

    let panel = Panel::new(temp_dir.path().to_path_buf())?;
    let alias = entry(&panel, "alias.txt");

    assert!(!alias.kind.is_dir());
    assert!(alias.kind.is_symlink());
    Ok(())
}

/// A link with nothing behind it cannot be entered, so it must not claim to be
/// a directory — `fs::metadata` fails and the entry stays a "file".
#[test]
fn a_broken_symlink_is_not_reported_as_a_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    symlink("../nowhere/gone", temp_dir.path().join("dangling")).expect("test setup");

    let panel = Panel::new(temp_dir.path().to_path_buf())?;
    let dangling = entry(&panel, "dangling");

    assert!(!dangling.kind.is_dir());
    assert!(dangling.kind.is_symlink());
    Ok(())
}

/// Plain entries must keep reporting exactly what they did before.
#[test]
fn plain_entries_are_not_flagged_as_links() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    fs::create_dir(temp_dir.path().join("dir")).expect("test setup");
    fs::write(temp_dir.path().join("file.txt"), b"1234").expect("test setup");

    let panel = Panel::new(temp_dir.path().to_path_buf())?;

    let dir = entry(&panel, "dir");
    assert!(dir.kind.is_dir());
    assert!(!dir.kind.is_symlink());

    let file = entry(&panel, "file.txt");
    assert!(!file.kind.is_dir());
    assert!(!file.kind.is_symlink());
    assert_eq!(file.size, FileSize::from_bytes(4));
    Ok(())
}

/// `load_entries` publishes the new listing only once it is complete: a panel
/// whose directory has vanished keeps showing what it last read successfully,
/// instead of being blanked down to a lone "..".
#[test]
fn a_failed_reload_keeps_the_previous_listing() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let dir = temp_dir.path().join("dir");
    fs::create_dir(&dir).expect("test setup");
    fs::write(dir.join("kept.txt"), b"payload").expect("test setup");

    let mut panel = Panel::new(dir.clone())?;
    let before = panel.entries.clone();

    fs::remove_file(dir.join("kept.txt")).expect("test setup");
    fs::remove_dir(&dir).expect("test setup");

    let err = panel.load_entries().expect_err("the directory is gone");

    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert_eq!(panel.entries.len(), before.len());
    assert!(
        panel.entries.iter().any(|entry| entry.name == "kept.txt"),
        "a failed reload must leave the panel exactly as it was"
    );
    Ok(())
}
