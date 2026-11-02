//! Purpose: Tests for the Windows drive mask and file attribute wrappers,
//! asked of the real system.

/// The system drive is mounted by definition, so its bit must be set.
#[cfg(windows)]
#[test]
fn logical_drives_include_the_system_drive() {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| String::from("C:"));
    let letter = drive
        .bytes()
        .next()
        .expect("a drive letter")
        .to_ascii_uppercase();
    let bit = u32::from(letter - b'A');
    assert_ne!(super::logical_drives() & (1 << bit), 0, "{drive} missing");
}

#[cfg(windows)]
#[test]
fn set_file_attributes_hides_and_restores_a_file() {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL};

    let dir = tempfile::TempDir::new().expect("tempdir");
    let file = dir.path().join("hide-me.txt");
    std::fs::write(&file, b"x").expect("write");
    let hidden = |path: &std::path::Path| {
        std::fs::metadata(path).expect("stat").file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    };
    super::set_file_attributes(&file, FILE_ATTRIBUTE_HIDDEN).expect("hide");
    assert!(hidden(&file));
    super::set_file_attributes(&file, FILE_ATTRIBUTE_NORMAL).expect("restore");
    assert!(!hidden(&file));
}

#[cfg(windows)]
#[test]
fn set_file_attributes_refuses_what_it_cannot_name() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let missing = super::set_file_attributes(&dir.path().join("absent"), 0x80);
    assert_eq!(
        missing.expect_err("no file").kind(),
        std::io::ErrorKind::NotFound
    );
    let nul = super::set_file_attributes(std::path::Path::new("bad\0name"), 0x80);
    assert_eq!(
        nul.expect_err("interior NUL").kind(),
        std::io::ErrorKind::InvalidInput
    );
}
