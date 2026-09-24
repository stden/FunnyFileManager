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

    // Test with empty extension
    File::create(temp_dir.path().join("empty."))?;
    let unique_empty_ext = unique_destination(temp_dir.path(), OsStr::new("empty."))?;
    assert!(unique_empty_ext.to_string_lossy().ends_with("empty_1"));
    assert!(!unique_empty_ext.to_string_lossy().ends_with("empty_1."));

    // Test with non-empty extension
    File::create(temp_dir.path().join("file.txt"))?;
    let unique_with_ext = unique_destination(temp_dir.path(), OsStr::new("file.txt"))?;
    assert!(unique_with_ext.to_string_lossy().ends_with("file_1.txt"));

    // Test the difference between empty and non-empty extensions
    assert!(unique_with_ext.to_string_lossy().contains('.'));
    assert!(!unique_empty_ext.to_string_lossy().ends_with('.'));

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

    // Should be seq_4.txt (counter incremented by 1)
    assert!(unique.to_string_lossy().contains("seq_4.txt"));

    // Should NOT be negative (would happen with -=)
    assert!(!unique.to_string_lossy().contains('-'));

    // Should NOT be exponential (would happen with *=)
    assert!(!unique.to_string_lossy().contains("seq_6.txt")); // 3 * 2
    assert!(!unique.to_string_lossy().contains("seq_9.txt")); // 3 * 3

    Ok(())
}
