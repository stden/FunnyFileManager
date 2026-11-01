//! Unit tests: the archive runs behind the three dialogs — the command each
//! format builds, the transcript line that keeps a strange name one word, the
//! archives `tar` really writes, and the members an extraction refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

#[cfg(unix)]
use super::command_line;
use super::members::{member_type, validate_listing};
use super::{
    create_archive, extract_archive, list_archive, offered_formats, on_path, test_archive,
    tests_extract, ArchiveRequest,
};
use crate::models::ArchiveFormat;

fn names(sources: &[&str]) -> Vec<String> {
    sources.iter().map(|name| (*name).to_owned()).collect()
}

mod adding;
mod reading;

#[test]
fn delete_after_removes_separate_source_directory_tree_and_files() {
    use crate::models::{App, ArchivePrompt, Panel};

    let dir = TempDir::new().expect("dir");
    let tree = dir.path().join("tree");
    let nested = tree.join("nested");
    fs::create_dir_all(&nested).expect("mkdir");
    fs::write(nested.join("leaf.txt"), b"leaf").expect("write");
    fs::write(dir.path().join("top.txt"), b"top").expect("write");

    let left = Panel::new(dir.path().to_path_buf()).expect("panel");
    let right = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(left, right);
    let mut prompt = ArchivePrompt::new(
        vec!["tree".to_owned(), "top.txt".to_owned()],
        "bundle",
        dir.path(),
        vec![ArchiveFormat::Tar],
    );
    prompt.format = ArchiveFormat::Tar;
    prompt.name.set_text("bundle.tar");
    prompt.delete_after = true;

    super::add::add(&mut app, &prompt);

    assert!(dir.path().join("bundle.tar").is_file());
    assert!(!tree.exists(), "source directory tree removed");
    assert!(!dir.path().join("top.txt").exists(), "source file removed");
}
