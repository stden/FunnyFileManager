//! Purpose: Archive guards that run before anything is unpacked or passed to a
//! tool: a setuid member is refused, an archive name that reads as an option is
//! given as a path, and an extraction answer with no prompt open does nothing.

use super::tool::safe_archive_arg;
use super::*;
use crate::models::{ArchiveCommand, FileName, Panel};
use tempfile::TempDir;

#[test]
fn a_plain_listing_is_accepted() {
    let details = "-rw-r--r-- root/root 1 2026-01-01 12:00 a.txt";

    assert!(validate_listing("a.txt", details).is_ok());
}

#[test]
fn a_member_with_setuid_bits_is_refused() {
    let details = "-rwsr-xr-x root/root 0 2026-01-01 12:00 bin/tool";

    let error = validate_listing("bin/tool", details).expect_err("a setuid member");

    assert!(error.to_string().contains("setuid"), "{error}");
}

#[test]
fn an_archive_name_that_reads_as_an_option_or_a_drive_is_given_as_a_path() {
    assert_eq!(
        safe_archive_arg(Path::new("-x.tar")).as_ref(),
        Path::new(".").join("-x.tar")
    );
    assert_eq!(
        safe_archive_arg(Path::new("host:x.tar")).as_ref(),
        Path::new(".").join("host:x.tar")
    );
    assert_eq!(
        safe_archive_arg(Path::new("plain.tar")).as_ref(),
        Path::new("plain.tar")
    );
}

#[test]
fn an_extraction_answer_with_no_prompt_open_changes_nothing() {
    let root = TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(root.path().to_path_buf()).unwrap(),
        Panel::new(root.path().to_path_buf()).unwrap(),
    );

    run_extract(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn run_command_reports_missing_archive_and_opens_add_prompt() {
    let temp = TempDir::new().unwrap();
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).unwrap(),
        Panel::new(temp.path().to_path_buf()).unwrap(),
    );
    run_command(&mut app, ArchiveCommand::ExtractHere);
    assert!(app.status_text().contains("put the cursor on an archive"));

    std::fs::write(temp.path().join("bundle.tar"), b"").unwrap();
    std::fs::write(temp.path().join("note.txt"), b"hi").unwrap();
    app.left_panel.load_entries().unwrap();
    assert!(app.left_panel.select_named("note.txt"));
    run_command(&mut app, ArchiveCommand::ExtractHere);
    assert!(matches!(app.overlay, Overlay::ExtractPrompt(_)));
    app.close_overlay();

    assert!(app.left_panel.select_named("bundle.tar"));
    app.left_panel.marked.insert(FileName::new("note.txt"));
    run_command(&mut app, ArchiveCommand::AddSelected);
    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
    app.close_overlay();

    app.left_panel.marked.insert(FileName::new("bundle.tar"));
    let root_path = temp.path().ancestors().last().unwrap().to_path_buf();
    app.left_panel.path = root_path;
    open_add(&mut app);
    assert!(matches!(app.overlay, Overlay::ArchivePrompt(_)));
    assert_eq!(commands::name_of(Path::new("/")), "/");
}

/// Several marks are named after their folder; the filesystem root has no
/// name, so the suggestion falls back to `archive`.
#[test]
fn multiple_marks_at_the_filesystem_root_use_the_archive_fallback_name() {
    let mut panel = Panel::from_entries(Path::new("/").into(), Vec::new());
    panel.marked.insert(FileName::new("first"));
    panel.marked.insert(FileName::new("second"));
    let mut app = App::with_panels(panel.clone(), panel);

    open_add(&mut app);

    let Overlay::ArchivePrompt(prompt) = &app.overlay else {
        panic!("archive prompt");
    };
    assert_eq!(
        Path::new(prompt.name.text())
            .file_name()
            .expect("archive name"),
        "archive.tar.gz"
    );
}
