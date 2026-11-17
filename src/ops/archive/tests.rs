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

#[test]
fn add_handles_existing_archive_tar_gz_and_tool_errors() {
    use crate::models::{App, ArchivePrompt, Panel};
    use crate::status::Status;
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("item.txt"), b"hello").expect("write");
    fs::write(dir.path().join("taken.tar"), b"existing").expect("write");
    let left = Panel::new(dir.path().to_path_buf()).expect("left");
    let right = Panel::new(dir.path().to_path_buf()).expect("right");
    let mut app = App::with_panels(left, right);
    let mut prompt = ArchivePrompt::new(
        names(&["item.txt"]),
        "taken",
        dir.path(),
        vec![ArchiveFormat::Tar],
    );
    prompt.format = ArchiveFormat::Tar;
    prompt.name.set_text("taken.tar");
    super::add::add(&mut app, &prompt);
    assert!(matches!(app.status, Status::FileExists(_)));
    prompt.format = ArchiveFormat::TarGz;
    prompt
        .name
        .set_text(&dir.path().join("out.tar.gz").display().to_string());
    super::add::add(&mut app, &prompt);
    assert!(dir.path().join("out.tar.gz").is_file());
    prompt.sources = names(&["missing_source.txt"]);
    prompt.name.set_text("bad.tar");
    super::add::add(&mut app, &prompt);
    assert!(matches!(app.status, Status::Error(_)));
}

#[test]
fn on_path_returns_false_when_path_env_is_unset() {
    let _lock = crate::test_support::env_lock();
    let prev = std::env::var_os("PATH");
    std::env::remove_var("PATH");
    let found = on_path("tar");
    if let Some(val) = prev {
        std::env::set_var("PATH", val);
    }
    assert!(!found);
}
