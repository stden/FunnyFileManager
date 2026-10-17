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
    let root = TempDir::new().unwrap();
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

/// Review finding: a canonicalized Windows panel path put `\\?\` into
/// `FFM_MCP_ROOT`; the same text rule now serves every caller, on every
/// platform.
#[test]
fn the_verbatim_prefix_goes_from_drives_and_shares_only() {
    for (raw, plain) in [
        (r"\\?\C:\work\проект", r"C:\work\проект"),
        (r"\\?\c:", "c:"),
        (r"\\?\UNC\server\share\x", r"\\server\share\x"),
        (r"\\?\Volume{example}\", r"\\?\Volume{example}\"),
        (r"\\?\", r"\\?\"),
        (r"\\.\COM1", r"\\.\COM1"),
        (r"C:\work", r"C:\work"),
        ("/srv/x/R&D", "/srv/x/R&D"),
    ] {
        assert_eq!(without_verbatim_prefix(raw), plain, "{raw}");
    }
}

/// Non-verbatim paths come back as they went in, on every platform.
#[test]
fn normalize_leaves_non_verbatim_paths_alone() {
    use std::path::PathBuf;

    for raw in [r"C:\work", "/srv/x"] {
        assert_eq!(normalize(PathBuf::from(raw)), PathBuf::from(raw), "{raw}");
    }
}

/// Windows strips the verbatim prefix from a `PathBuf`.
#[cfg(windows)]
#[test]
fn normalize_strips_verbatim_prefix_from_a_path_buf() {
    use std::path::PathBuf;

    for (raw, expected) in [
        (r"\\?\C:\work\проект", r"C:\work\проект"),
        (r"\\?\UNC\server\share\x", r"\\server\share\x"),
    ] {
        assert_eq!(
            normalize(PathBuf::from(raw)),
            PathBuf::from(expected),
            "{raw}"
        );
    }
}

/// Off Windows there is no verbatim prefix to strip, so the spelling stays.
#[cfg(not(windows))]
#[test]
fn normalize_returns_verbatim_spellings_unchanged_off_windows() {
    use std::path::PathBuf;

    for raw in [r"\\?\C:\work\проект", r"\\?\UNC\server\share\x"] {
        assert_eq!(normalize(PathBuf::from(raw)), PathBuf::from(raw), "{raw}");
    }
}

#[test]
fn normalize_does_not_touch_device_or_volume_paths() {
    use std::path::PathBuf;

    for raw in [r"\\?\Volume{example}\", r"\\?\", r"\\.\COM1"] {
        assert_eq!(normalize(PathBuf::from(raw)), PathBuf::from(raw), "{raw}");
    }
}
