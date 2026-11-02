//! Purpose: Verify transfer dialogs expose only their destination or archive name to paste.

use super::{app, paste};
use crate::copy::{CopyField, CopyPrompt};
use crate::models::{ArchiveField, ArchivePrompt, ExtractField, ExtractPrompt, Overlay};
use tempfile::TempDir;

#[test]
fn copy_paste_changes_the_destination_only_while_that_field_is_focused() {
    let root = TempDir::new().unwrap();
    let prompt = CopyPrompt::new("source.txt", root.path());
    let mut app = app(&root, Overlay::CopyPrompt(Box::new(prompt)));
    paste(&mut app, "/target");
    let Overlay::CopyPrompt(prompt) = &mut app.overlay else {
        panic!("the copy prompt stays open");
    };
    let destination = prompt.destination.text().to_owned();
    assert!(destination.ends_with("/target"));
    prompt.field = CopyField::Copy;
    assert!(app.focused_text_field().is_none());
    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("the copy prompt stays open");
    };
    assert_eq!(prompt.what, "source.txt");
    assert_eq!(prompt.destination.text(), destination);
}

#[test]
fn archive_paste_changes_the_name_and_leaves_the_format_and_sources_alone() {
    let root = TempDir::new().unwrap();
    let prompt = ArchivePrompt {
        name: "bundle".into(),
        sources: vec!["source.txt".into()],
        ..ArchivePrompt::default()
    };
    let mut app = app(&root, Overlay::ArchivePrompt(Box::new(prompt)));
    paste(&mut app, ".tar.gz");
    let Overlay::ArchivePrompt(prompt) = &mut app.overlay else {
        panic!("the archive prompt stays open");
    };
    assert_eq!(prompt.name.text(), "bundle.tar.gz");
    assert_eq!(prompt.sources, ["source.txt"]);
    assert_eq!(prompt.format, crate::models::ArchiveFormat::TarGz);
    prompt.field = ArchiveField::Format;
    assert!(app.focused_text_field().is_none());
}

#[test]
fn extraction_paste_changes_the_destination_and_never_edits_a_button() {
    let root = TempDir::new().unwrap();
    let archive = root.path().join("bundle.zip");
    let mut prompt = ExtractPrompt::new(vec![archive.clone()], root.path());
    prompt.destination.set_text("target");
    let mut app = app(&root, Overlay::ExtractPrompt(Box::new(prompt)));
    paste(&mut app, "/unpacked");
    let Overlay::ExtractPrompt(prompt) = &mut app.overlay else {
        panic!("the extraction prompt stays open");
    };
    assert_eq!(prompt.destination.text(), "target/unpacked");
    assert_eq!(prompt.archives, [archive]);
    prompt.field = ExtractField::Cancel;
    assert!(app.focused_text_field().is_none());
}
