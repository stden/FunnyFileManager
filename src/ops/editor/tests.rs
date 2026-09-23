//! Unit tests: `Editor::save` when the target file does not exist yet, and
//! when the path the editor was handed is not the file itself.

use std::{fs, io};

use tempfile::TempDir;

use crate::models::*;

fn dir_names(dir: &std::path::Path) -> io::Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)?
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn test_save_creates_a_target_that_does_not_exist_yet() -> io::Result<()> {
    // Shift+F4 hands the editor a path with nothing behind it. `fs::metadata`
    // then fails, and the permission-copy step has to be skipped rather than
    // propagated — otherwise the first save of a new file returns NotFound.
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("brand-new.txt");
    assert!(!path.exists(), "test setup: target must not exist yet");

    let mut editor = Editor::from_content("brand-new.txt".to_owned(), path.clone(), "");
    editor.insert_char('h');
    editor.insert_char('i');
    assert!(editor.state.is_modified());

    editor.save()?;

    assert_eq!(fs::read_to_string(&path)?, "hi");
    assert!(
        !editor.state.is_modified(),
        "a successful save clears the dirty flag"
    );
    // The temp file was renamed onto the target, not left beside it.
    assert_eq!(dir_names(temp_dir.path())?, ["brand-new.txt"]);
    Ok(())
}

/// The `~/.vimrc -> ~/dotfiles/vimrc` layout: persisting the temp file is a
/// rename, so without resolving the path first it replaced the *link* with a
/// regular file and left the real, version-controlled file holding the old text.
#[cfg(unix)]
#[test]
fn test_save_writes_through_a_symlink_instead_of_replacing_it() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().expect("test setup");
    let dotfiles = temp_dir.path().join("dotfiles");
    fs::create_dir(&dotfiles).expect("test setup");
    let real = dotfiles.join("vimrc");
    fs::write(&real, "set nocompatible\n").expect("test setup");
    let link = temp_dir.path().join(".vimrc");
    symlink(&real, &link).expect("test setup");

    let mut editor = Editor::from_content(".vimrc".to_owned(), link.clone(), "set nocompatible\n");
    editor.move_end();
    editor.insert_char('!');
    editor.save()?;

    // `Path::exists` follows the link and would pass either way.
    assert!(
        fs::symlink_metadata(&link)?.is_symlink(),
        "the link must still be a link after a save"
    );
    assert_eq!(fs::read_to_string(&real)?, "set nocompatible!\n");
    assert_eq!(fs::read_to_string(&link)?, "set nocompatible!\n");
    // No temp file was left in the link's directory.
    assert_eq!(dir_names(temp_dir.path())?, [".vimrc", "dotfiles"]);
    Ok(())
}

/// A relative link target resolves against the link's directory, not the
/// process CWD — `canonicalize` handles that; a hand-rolled `read_link` would
/// not.
#[cfg(unix)]
#[test]
fn test_save_follows_a_relative_symlink() -> io::Result<()> {
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().expect("test setup");
    let real = temp_dir.path().join("real.txt");
    fs::write(&real, "one\n").expect("test setup");
    let link = temp_dir.path().join("alias.txt");
    symlink("real.txt", &link).expect("test setup");

    let mut editor = Editor::from_content("alias.txt".to_owned(), link.clone(), "one\n");
    editor.move_end();
    editor.insert_char('!');
    editor.save()?;

    assert!(fs::symlink_metadata(&link)?.is_symlink());
    assert_eq!(fs::read_to_string(&real)?, "one!\n");
    Ok(())
}

/// A hard link has no "real" name to canonicalise, so the rename would detach
/// the name being saved and leave every other name on the pre-edit inode. Such
/// files are rewritten in place instead.
#[cfg(unix)]
#[test]
fn test_save_keeps_hard_linked_names_in_sync() -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let temp_dir = TempDir::new().expect("test setup");
    let first = temp_dir.path().join("first.txt");
    fs::write(&first, "shared\n").expect("test setup");
    let second = temp_dir.path().join("second.txt");
    fs::hard_link(&first, &second).expect("test setup");
    let inode_before = fs::metadata(&first)?.ino();

    let mut editor = Editor::from_content("first.txt".to_owned(), first.clone(), "shared\n");
    editor.move_end();
    editor.insert_char('!');
    editor.save()?;

    assert_eq!(fs::read_to_string(&first)?, "shared!\n");
    assert_eq!(
        fs::read_to_string(&second)?,
        "shared!\n",
        "the other name must see the edit, not the pre-edit text"
    );
    assert_eq!(
        fs::metadata(&first)?.ino(),
        inode_before,
        "a hard-linked file is rewritten in place, so the inode is kept"
    );
    assert_eq!(fs::metadata(&first)?.nlink(), 2, "the link must survive");
    Ok(())
}

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

/// The ordinary save writes a sibling temp file and renames it over the
/// target: `fs::write` truncates the original before the first new byte
/// lands, so a full disk mid-write would leave nothing but an empty file.
#[cfg(unix)]
#[test]
fn a_save_replaces_the_file_and_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;

    use crate::models::Editor;

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("notes.txt");
    fs::write(&path, "old\n").expect("test setup");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("test setup");

    let mut editor = Editor::from_content("notes.txt".to_owned(), path.clone(), "new\n");
    editor.save().expect("save");

    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o640,
        "the original's mode survived the rename"
    );
}

/// A file with another name pointing at the same inode is rewritten in place
/// instead: renaming over it would break the other name, which is the whole
/// reason the two paths exist.
#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_rewritten_in_place_so_the_other_name_survives() {
    use crate::models::Editor;

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("notes.txt");
    let other = dir.path().join("also-notes.txt");
    fs::write(&path, "old\n").expect("test setup");
    fs::hard_link(&path, &other).expect("test setup");

    let mut editor = Editor::from_content("notes.txt".to_owned(), path.clone(), "new\n");
    editor.save().expect("save");

    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    assert_eq!(
        fs::read_to_string(&other).unwrap(),
        "new\n",
        "the other name sees the change, which a rename would have broken"
    );
}

/// A save into a directory that is gone reports the failure rather than
/// quietly marking the buffer clean.
#[test]
fn a_save_that_cannot_be_written_leaves_the_buffer_modified() {
    use crate::models::{EditState, Editor};

    let dir = TempDir::new().expect("test setup");
    let path = dir.path().join("gone/notes.txt");
    let mut editor = Editor::from_content("notes.txt".to_owned(), path, "new\n");
    editor.state = EditState::Modified;

    assert!(editor.save().is_err());
    assert_ne!(editor.state, EditState::Clean, "still unsaved");
}

struct OscClipboard;

impl crate::clipboard::ClipboardBackend for OscClipboard {
    fn get_native(&mut self) -> Result<String, String> {
        Err("no native clipboard".to_owned())
    }
    fn set_native(&mut self, _: &str) -> Result<(), String> {
        Err("no native clipboard".to_owned())
    }
    fn emit_osc52(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

fn editor_app(content: &str, path: std::path::PathBuf) -> App {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "t.txt".to_owned(),
        path,
        content,
    )));
    app
}

#[test]
fn copying_falls_back_to_the_terminal_clipboard() {
    let dir = TempDir::new().expect("dir");
    let mut app = editor_app("ab", dir.path().join("t.txt"));
    app.clipboard.install(Box::new(OscClipboard));
    app.get_editor_mut().expect("editor").select_all();
    crate::ops::copy_editor_selection(&mut app);
    let notice = app.get_editor().expect("editor").notice.clone();
    assert!(
        notice
            .as_deref()
            .is_some_and(|text| text.contains("terminal")),
        "{notice:?}"
    );
}

#[test]
fn paste_and_copy_do_nothing_without_an_editor() {
    let mut app = App::new().expect("app");
    crate::ops::copy_editor_selection(&mut app);
    crate::ops::paste_into_editor(&mut app);
    assert!(app.get_editor().is_none());
}

#[test]
fn reloading_a_directory_as_text_is_refused() {
    let dir = TempDir::new().expect("dir");
    let mut app = editor_app("x", dir.path().to_path_buf());
    crate::ops::reload_editor_encoding(&mut app, crate::encoding::TextEncoding::Utf8);
    let notice = app.get_editor().expect("editor").notice.clone();
    assert!(notice.is_some(), "{notice:?}");
}

/// Whether the suite is running with root's ability to ignore permissions.
/// Root is allowed to save a read-only file, so the refusal cannot be asserted
/// under it; the tests below say so rather than passing vacuously.
#[cfg(unix)]
fn running_as_root() -> bool {
    crate::platform::is_root()
}

/// Renaming a fresh file over the target needs write permission on the
/// *directory*, not on the file, so a read-only file used to be replaced in
/// silence — a new inode, and, when the file belonged to somebody else, a new
/// owner too. The editor said "Saved" either way.
#[cfg(unix)]
#[test]
fn test_save_refuses_a_file_the_user_cannot_write() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new()?;
    let path = dir.path().join("locked.txt");
    fs::write(&path, "ORIGINAL\n")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444))?;

    let mut editor = Editor::from_content("locked.txt".to_owned(), path.clone(), "EDITED\n");

    if running_as_root() {
        editor.save()?;
        assert_eq!(
            fs::read_to_string(&path)?,
            "EDITED\n",
            "root may write a read-only file, and must not be refused"
        );
        return Ok(());
    }

    let error = editor
        .save()
        .expect_err("a file the user cannot write must not be replaced");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(
        fs::read_to_string(&path)?,
        "ORIGINAL\n",
        "the refusal must leave the file exactly as it was"
    );
    Ok(())
}

/// The guard must not cost anyone an ordinary save: a writable file still goes
/// through the atomic path, and a file that does not exist yet is still
/// created.
#[test]
fn test_save_still_writes_a_writable_file() -> io::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("plain.txt");
    fs::write(&path, "ORIGINAL\n")?;

    let mut editor = Editor::from_content("plain.txt".to_owned(), path.clone(), "EDITED\n");
    editor.save()?;

    assert_eq!(fs::read_to_string(&path)?, "EDITED\n");
    Ok(())
}
