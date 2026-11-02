//! Purpose: unit tests for how `unique_destination` spells a free name: stem, extension and folder.

use super::*;

use std::ffi::OsStr;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn test_unique_destination_no_conflict() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    let file_name = OsStr::new("test.txt");

    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test.txt"));
}

#[test]
fn test_unique_destination_with_conflicts() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create existing files
    File::create(base.join("test.txt")).unwrap();
    File::create(base.join("test_1.txt")).unwrap();

    let file_name = OsStr::new("test.txt");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test_2.txt"));
}

#[test]
fn test_unique_destination_no_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test")).unwrap();

    let file_name = OsStr::new("test");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test_1"));
}

#[test]
fn test_unique_destination_multiple_dots() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test.backup.txt")).unwrap();

    let file_name = OsStr::new("test.backup.txt");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join("test.backup_1.txt"));
}

#[test]
fn test_unique_destination_empty_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("test.")).unwrap();

    let file_name = OsStr::new("test.");
    let dest = unique_destination(base, file_name).unwrap();
    // When extension is empty, it's treated as no extension
    assert_eq!(dest, base.join("test_1"));
}

#[test]
fn test_unique_destination_dotfile() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join(".env")).unwrap();

    let file_name = OsStr::new(".env");
    let dest = unique_destination(base, file_name).unwrap();
    assert_eq!(dest, base.join(".env_1"));
}

#[test]
fn test_unique_destination_unicode_name() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("данные.txt")).unwrap();

    let dest = unique_destination(base, OsStr::new("данные.txt")).unwrap();
    assert_eq!(dest, base.join("данные_1.txt"));
}

#[test]
fn test_unique_destination_hidden_file_with_extension() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join(".config.json")).unwrap();

    let dest = unique_destination(base, OsStr::new(".config.json")).unwrap();
    assert_eq!(dest, base.join(".config_1.json"));
}

#[test]
fn test_unique_destination_preserves_base_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested = temp_dir.path().join("nested");
    fs::create_dir(&nested).unwrap();
    File::create(nested.join("image.png")).unwrap();

    let dest = unique_destination(&nested, OsStr::new("image.png")).unwrap();
    assert!(dest.starts_with(&nested));
    assert_eq!(dest, nested.join("image_1.png"));
}

#[test]
fn test_unique_destination_special_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let without_ext = temp_dir.path().join("noext");
    File::create(&without_ext).expect("test setup");
    let unique = unique_destination(temp_dir.path(), without_ext.file_name().unwrap())?;
    assert!(unique.to_string_lossy().contains("noext_1"));

    let multiple_dots = temp_dir.path().join("file.tar.gz");
    File::create(&multiple_dots).expect("test setup");
    let unique = unique_destination(temp_dir.path(), multiple_dots.file_name().unwrap())?;
    assert!(unique.to_string_lossy().contains("file.tar_1.gz"));

    let hidden = temp_dir.path().join(".hidden");
    File::create(&hidden).expect("test setup");
    let unique = unique_destination(temp_dir.path(), hidden.file_name().unwrap())?;
    assert!(unique.to_string_lossy().contains(".hidden_1"));

    let trailing_dot = temp_dir.path().join("file.");
    File::create(&trailing_dot).expect("test setup");
    let unique = unique_destination(temp_dir.path(), trailing_dot.file_name().unwrap())?;
    assert!(unique.to_string_lossy().contains("file_1"));

    let base_name = temp_dir.path().join("test");
    for i in 0..15 {
        if i == 0 {
            File::create(&base_name).expect("test setup");
        } else {
            File::create(temp_dir.path().join(format!("test_{i}"))).expect("test setup");
        }
    }
    let unique = unique_destination(temp_dir.path(), base_name.file_name().unwrap())?;
    assert!(unique.to_string_lossy().contains("test_15"));

    Ok(())
}
