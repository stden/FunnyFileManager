//! Purpose: Saves the user may not write, read-only folders, and dangling links.

use super::*;

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

/// A directory the user may not add to (0555), holding a file they may write.
/// Returns the directory; `restore` gives it back its write bit so `TempDir`
/// can clean up.
#[cfg(unix)]
fn read_only_dir(root: &std::path::Path) -> io::Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let locked = root.join("read-only-dir");
    fs::create_dir(&locked)?;
    fs::write(locked.join("file.txt"), "ORIGINAL\n")?;
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555))?;
    Ok(locked)
}

#[cfg(unix)]
fn restore(locked: &std::path::Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(locked, fs::Permissions::from_mode(0o755))
}

/// The atomic save needs a temp file beside the target, which a read-only
/// directory refuses, and the save failed with "Permission denied … at path
/// …/.tmpXXXXXX" although the file itself was writable. It now writes
/// through the file, as vim and nano do. (Root may create files anywhere, so
/// under root this only shows the save still works.)
#[cfg(unix)]
#[test]
fn a_writable_file_in_a_read_only_directory_is_saved_in_place() -> io::Result<()> {
    let dir = TempDir::new()?;
    let locked = read_only_dir(dir.path())?;
    let path = locked.join("file.txt");
    let mut editor = Editor::from_content("file.txt".to_owned(), path.clone(), "EDITED\n");
    editor.state = EditState::Modified;

    let result = editor.save();
    let listing = dir_names(&locked);
    restore(&locked)?;

    result?;
    assert_eq!(fs::read_to_string(&path)?, "EDITED\n");
    assert_eq!(listing?, ["file.txt"], "no temp file is left behind");
    Ok(())
}

/// When the save really cannot happen — a new file in a read-only directory —
/// the message names the directory that refused, never the random temp file
/// the user has never heard of.
#[cfg(unix)]
#[test]
fn a_refused_save_names_the_directory_not_a_temp_file() -> io::Result<()> {
    if running_as_root() {
        return Ok(()); // Root may create a file in a read-only directory.
    }
    let dir = TempDir::new()?;
    let locked = read_only_dir(dir.path())?;
    let mut editor = Editor::from_content("new.txt".to_owned(), locked.join("new.txt"), "");
    editor.state = EditState::Modified;

    let result = editor.save();
    restore(&locked)?;

    let message = result
        .expect_err("nothing can be created there")
        .to_string();
    assert!(
        !message.contains("read-only-dir/.tmp"),
        "no temp file name: {message}"
    );
    assert!(message.contains("read-only-dir"), "{message}");
    Ok(())
}

/// A link whose target is gone: a rename would replace the link with a
/// regular file, so the save is refused and the link is left as it was.
#[cfg(unix)]
#[test]
fn saving_through_a_dangling_link_is_refused_and_the_link_is_kept() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let missing = temp_dir.path().join("gone.txt");
    let link = temp_dir.path().join("notes.txt");
    std::os::unix::fs::symlink(&missing, &link)?;

    let mut editor = Editor::from_content("notes.txt".to_owned(), link.clone(), "");
    editor.insert_char('x');
    let error = editor.save().expect_err("a dangling link is refused");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(error.to_string().contains("gone.txt"), "{error}");
    assert!(fs::symlink_metadata(&link)?.is_symlink());
    assert!(!missing.exists());
    assert!(editor.state.is_modified(), "the buffer keeps its changes");
    Ok(())
}
