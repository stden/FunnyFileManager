//! Purpose: Where a save lands: a new file, through a symlink, and onto hard-linked names.

use super::*;

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
