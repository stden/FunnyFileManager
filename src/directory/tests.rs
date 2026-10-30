//! Filesystem adapter tests use isolated directories and real metadata.

use tempfile::TempDir;

use super::*;

#[test]
fn reading_captures_entries_parent_and_capacity_together() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("notes.txt"), b"notes")?;
    fs::create_dir(temp.path().join("documents"))?;

    let listing = read(temp.path(), &crate::description::default_names())?;
    assert_eq!(listing.entries.len(), 2);
    let note = listing
        .entries
        .iter()
        .find(|entry| entry.name == "notes.txt")
        .expect("file listed");
    assert_eq!(note.kind, EntryKind::File);
    assert_eq!(note.size, FileSize::from_bytes(5));
    assert_eq!(
        note.modified,
        Modified::from_metadata(&fs::metadata(temp.path().join("notes.txt"))?)
    );
    assert!(!note.executable);
    assert_eq!(listing.parent.expect("parent row").name, "..");
    assert!(!listing.metadata.hidden_names.contains("notes.txt"));
    #[cfg(any(unix, windows))]
    assert!(listing.metadata.available_space.is_some());
    Ok(())
}

#[test]
fn cmd_sh_and_py_scripts_are_marked_executable_while_directories_are_not() -> io::Result<()> {
    let temp = TempDir::new()?;
    for script in ["build.cmd", "RUN.SH", "tool.Py"] {
        fs::write(temp.path().join(script), b"echo ok\n")?;
    }
    fs::write(temp.path().join("readme.md"), b"# doc\n")?;
    fs::create_dir(temp.path().join("folder.py"))?;

    let listing = read(temp.path(), &[])?;
    let flag = |name: &str| {
        listing
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .expect("entry listed")
            .executable
    };
    assert!(flag("build.cmd"));
    assert!(flag("RUN.SH"));
    assert!(flag("tool.Py"));
    assert!(!flag("readme.md"));
    assert!(!flag("folder.py"));
    Ok(())
}

#[test]
fn an_unreadable_directory_returns_an_error() -> io::Result<()> {
    let temp = TempDir::new()?;
    let missing = temp.path().join("missing");
    assert_eq!(
        read(&missing, &crate::description::default_names())
            .expect_err("missing")
            .kind(),
        io::ErrorKind::NotFound
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn hidden_attributes_change_only_when_the_panel_reloads() -> io::Result<()> {
    use crate::platform::set_file_attributes;
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL};

    let temp = TempDir::new()?;
    let file = temp.path().join("secret.txt");
    fs::write(&file, b"data")?;
    let mut panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    assert!(!panel.is_hidden("secret.txt"));

    set_file_attributes(&file, FILE_ATTRIBUTE_HIDDEN)?;
    assert!(
        !panel.is_hidden("secret.txt"),
        "the loaded snapshot is unchanged"
    );
    panel.load_entries()?;
    assert!(panel.is_hidden("secret.txt"));

    set_file_attributes(&file, FILE_ATTRIBUTE_NORMAL)?;
    assert!(panel.is_hidden("secret.txt"));
    panel.load_entries()?;
    assert!(!panel.is_hidden("secret.txt"));
    Ok(())
}

#[test]
fn the_richer_facts_come_from_the_same_metadata() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("notes.txt"), b"notes")?;
    fs::write(temp.path().join("Descript.ion"), "notes.txt A note\n")?;
    let listing = read(temp.path(), &crate::description::default_names())?;
    let note = listing
        .entries
        .iter()
        .find(|entry| entry.name == "notes.txt")
        .expect("file listed");
    assert_eq!(note.description.as_deref(), Some("A note"));
    assert_eq!(note.links, 1);
    assert!(
        note.allocated >= note.size,
        "allocation rounds up to blocks"
    );
    assert_eq!(
        note.attributes,
        Attributes::from_metadata(&fs::metadata(temp.path().join("notes.txt"))?)
    );
    let listing = read(temp.path(), &[])?;
    assert!(listing
        .entries
        .iter()
        .all(|entry| entry.description.is_none()));
    Ok(())
}

#[cfg(unix)]
#[test]
fn unix_reports_the_owner_the_mode_and_the_access_time() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("notes.txt"), b"notes")?;
    let listing = read(temp.path(), &[])?;
    let note = &listing.entries[0];
    assert!(note.owner.is_some(), "the running user has a passwd entry");
    assert!(note.attributes.to_string().starts_with("-rw"));
    assert_ne!(note.accessed, Modified::UNKNOWN, "atime is always reported");
    Ok(())
}

/// A folder deleted with its parents leads to the first one above that is
/// still there; one that exists is its own answer.
#[test]
fn the_nearest_existing_folder_skips_every_deleted_level() -> io::Result<()> {
    let temp = TempDir::new()?;
    let deep = temp.path().join("a").join("b").join("c");
    assert_eq!(nearest_existing(&deep), Some(temp.path()));
    assert_eq!(nearest_existing(temp.path()), Some(temp.path()));

    let file = temp.path().join("file.txt");
    fs::write(&file, "not a folder")?;
    assert_eq!(nearest_existing(&file.join("sub")), Some(temp.path()));
    Ok(())
}

/// `link_count` is `st_nlink`: one for a lone file, one more for every hard
/// link — the number Wipe checks before it zeroes anything.
#[cfg(unix)]
#[test]
fn link_count_counts_every_name_of_a_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("hl-a");
    fs::write(&file, b"data")?;
    assert_eq!(link_count(&fs::symlink_metadata(&file)?), 1);

    fs::hard_link(&file, temp.path().join("hl-b"))?;

    assert_eq!(link_count(&fs::symlink_metadata(&file)?), 2);
    Ok(())
}
