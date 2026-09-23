//! Purpose: Verify ordinary Windows path labels and retain all filesystem path information.

use super::*;
use tempfile::TempDir;

#[test]
fn ordinary_paths_and_unicode_names_are_preserved() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("проект 😀.rs");
    assert_eq!(label(&path), path.to_string_lossy());
    assert_eq!(label(Path::new("relative/file.rs")), "relative/file.rs");
}

#[cfg(windows)]
#[test]
fn extended_drive_and_unc_paths_have_normal_labels() {
    for (raw, display) in [
        (r"\\?\C:\work\проект.rs", r"C:\work\проект.rs"),
        (r"\\?\UNC\server\share\file.rs", r"\\server\share\file.rs"),
        (r"\\?\C:\", r"C:\"),
    ] {
        let path = Path::new(raw);
        assert_eq!(label(path), display);
        assert_eq!(
            path.as_os_str(),
            raw,
            "displaying a path must not mutate it"
        );
    }
}

#[cfg(windows)]
#[test]
fn device_and_volume_names_are_not_misrepresented_as_normal_paths() {
    for raw in [r"\\.\COM1", r"\\?\Volume{example}\", r"\\server\share"] {
        assert_eq!(label(Path::new(raw)), raw);
    }
}

#[cfg(windows)]
#[test]
fn console_directory_keeps_native_names_and_regular_prefixes() {
    use crate::windows::console_directory;
    let root = TempDir::new().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    assert_eq!(
        console_directory(&canonical).canonicalize().unwrap(),
        canonical
    );
    assert_eq!(
        console_directory(Path::new(r"\\?\UNC\server\share")),
        Path::new(r"\\server\share")
    );
    assert_eq!(
        console_directory(Path::new(r"C:\work")),
        Path::new(r"C:\work")
    );
    assert_eq!(
        console_directory(Path::new("relative")),
        Path::new("relative")
    );
}

#[cfg(unix)]
#[test]
fn unix_backslashes_are_literal_filename_characters() {
    let raw = r"\\?\C:\file";
    assert_eq!(label(Path::new(raw)), raw);
}
#[cfg(windows)]
#[test]
fn status_paths_use_ordinary_spelling_even_when_stored_as_strings() {
    let root = tempfile::TempDir::new().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    let raw = canonical.to_string_lossy().into_owned();
    for status in [
        crate::status::Status::Viewing(raw.clone()),
        crate::status::Status::Editing(raw.clone()),
        crate::status::Status::Edited(raw.clone()),
        crate::status::Status::CreatedDir(raw.clone()),
        crate::status::Status::BinaryFile(raw.clone()),
        crate::status::Status::NotRegularFile(raw),
    ] {
        assert!(!status.to_string().contains(r"\\?\"));
        assert!(status.to_string().contains(label(&canonical).as_ref()));
    }
}
