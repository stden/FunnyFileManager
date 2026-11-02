//! Purpose: The exit dialog, changes on disk, and the detected file format.

use super::*;

#[test]
fn the_exit_dialog_is_a_stage_of_being_modified() {
    // `modified: bool` beside `confirm_exit: bool` made "asking whether to
    // save, with nothing to save" representable. The dialog exists *because*
    // there are unsaved changes, so it is a stage of the same state.
    assert!(!EditState::Clean.is_modified());
    assert!(!EditState::Clean.is_confirming_exit());

    assert!(EditState::Modified.is_modified());
    assert!(!EditState::Modified.is_confirming_exit());

    assert!(
        EditState::ConfirmingExit(ExitChoice::default()).is_modified(),
        "the buffer still differs from the file while the dialog is up"
    );
    assert!(EditState::ConfirmingExit(ExitChoice::default()).is_confirming_exit());
    assert_eq!(EditState::default(), EditState::Clean);

    // The "changed on disk" question is asked by a save, so it is a stage of
    // being modified too — and only it carries the question.
    let asking = EditState::ConfirmingOverwrite(OverwritePrompt::new(AfterSave::Close));
    assert!(asking.is_modified());
    assert_eq!(
        asking.overwrite_prompt().map(|prompt| prompt.then),
        Some(AfterSave::Close)
    );
    assert_eq!(EditState::Modified.overwrite_prompt(), None);
}

/// Another program's write is a change; the editor's own save is not; a
/// buffer that was never read from disk has nothing to compare with.
#[test]
fn changed_on_disk_tells_another_programs_write_from_the_editors_own() -> io::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("notes.txt");
    fs::write(&path, "original\n")?;
    let mut editor = Editor::from_content("notes.txt".to_owned(), path.clone(), "original\n");
    assert!(!editor.changed_on_disk(), "never read: nothing to compare");

    editor.disk = Some(Fingerprint::of(b"original\n"));
    assert!(!editor.changed_on_disk());
    fs::write(&path, "external change\n")?;
    assert!(editor.changed_on_disk(), "another program wrote it");

    editor.state = EditState::Modified;
    editor.save()?;
    assert!(!editor.changed_on_disk(), "the editor's own save is known");
    fs::remove_file(&path)?;
    assert!(editor.changed_on_disk(), "a file that is gone has changed");
    Ok(())
}

#[test]
fn the_file_format_is_detected_and_restored_as_one_thing() {
    // The EOL style and the final newline were separate fields, read together,
    // written together and detected together. One value keeps them in step.
    let crlf = FileFormat::detect("a\r\nb\r\n");
    assert_eq!(crlf.line_ending, LineEnding::Crlf);
    assert!(crlf.trailing_newline);

    let lf_no_final = FileFormat::detect("a\nb");
    assert_eq!(lf_no_final.line_ending, LineEnding::Lf);
    assert!(!lf_no_final.trailing_newline);

    let empty = FileFormat::detect("");
    assert_eq!(empty, FileFormat::default());
    assert!(!empty.trailing_newline);
}

// ─── the exit dialog's ring, and the two ways a save is written ─────────────

/// Three named answers, so the ring has to turn both ways and come back.
#[test]
fn the_exit_dialog_walks_both_ways_round_its_three_answers() {
    use crate::models::ExitChoice;

    let start = ExitChoice::default();
    let mut choice = start;
    for _ in 0..3 {
        choice = choice.previous();
    }
    assert_eq!(choice, start, "a full turn backwards");
    assert_eq!(start.next().previous(), start, "one step and back");
    assert_ne!(start.previous(), start.next(), "three answers, not two");
}
