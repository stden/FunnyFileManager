//! Purpose: Unique destination names: defaults, extensions and counters.

use super::*;

#[test]
fn test_unique_destination_not_default() -> io::Result<()> {
    // Kill: replace unique_destination -> PathBuf with Default::default()
    let temp_dir = TempDir::new()?;
    let result = unique_destination(temp_dir.path(), OsStr::new("test.txt"))?;

    assert!(!result.as_os_str().is_empty());
    assert_ne!(result, PathBuf::default());
    assert_ne!(result, PathBuf::new());
    assert!(result.starts_with(temp_dir.path()));

    Ok(())
}

#[test]
fn test_unique_destination_extension_guard() -> io::Result<()> {
    // Kill: replace match guard !ext.is_empty() with true
    // Kill: replace match guard !ext.is_empty() with false
    // Kill: delete ! in unique_destination
    let temp_dir = TempDir::new()?;

    // Only the file name is checked: the temporary directory's own path may
    // hold a '-' or a '.', and says nothing about the name chosen.
    // Test with empty extension: no dot is added back.
    File::create(temp_dir.path().join("empty."))?;
    let unique_empty_ext = unique_destination(temp_dir.path(), OsStr::new("empty."))?;
    assert_eq!(unique_empty_ext.file_name(), Some(OsStr::new("empty_1")));

    // Test with non-empty extension: the extension is kept.
    File::create(temp_dir.path().join("file.txt"))?;
    let unique_with_ext = unique_destination(temp_dir.path(), OsStr::new("file.txt"))?;
    assert_eq!(unique_with_ext.file_name(), Some(OsStr::new("file_1.txt")));

    Ok(())
}

#[test]
fn test_unique_destination_counter_increment() -> io::Result<()> {
    // Kill: replace += with -= in counter increment
    // Kill: replace += with *= in counter increment
    let temp_dir = TempDir::new()?;

    // Create a series of files
    File::create(temp_dir.path().join("seq.txt"))?;
    File::create(temp_dir.path().join("seq_1.txt"))?;
    File::create(temp_dir.path().join("seq_2.txt"))?;
    File::create(temp_dir.path().join("seq_3.txt"))?;

    let unique = unique_destination(temp_dir.path(), OsStr::new("seq.txt"))?;

    // seq_4.txt: counter incremented by 1 — not negative (-=), not
    // exponential (*=). Only the file name is checked: the temporary
    // directory's own path may hold a '-'.
    assert_eq!(unique.file_name(), Some(OsStr::new("seq_4.txt")));

    Ok(())
}
