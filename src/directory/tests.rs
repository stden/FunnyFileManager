//! Filesystem adapter tests use isolated directories and real metadata.

use std::fs;

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
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    };

    let temp = TempDir::new()?;
    let file = temp.path().join("secret.txt");
    fs::write(&file, b"data")?;
    let wide: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    assert!(!panel.is_hidden("secret.txt"));

    // SAFETY: `wide` is a NUL-terminated path and outlives each call.
    assert_ne!(
        unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN) },
        0
    );
    assert!(
        !panel.is_hidden("secret.txt"),
        "the loaded snapshot is unchanged"
    );
    panel.load_entries()?;
    assert!(panel.is_hidden("secret.txt"));

    // SAFETY: same owned, NUL-terminated path as above.
    assert_ne!(
        unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_NORMAL) },
        0
    );
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
