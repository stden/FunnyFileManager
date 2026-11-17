//! Purpose: Size formatting and unique destination names.

use super::*;

#[test]
fn test_format_size_all_ranges() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(999).to_string(), "999 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_572_864).to_string(), "1.50 MB");
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(FileSize::from_bytes(1_610_612_736).to_string(), "1.50 GB");
    assert_eq!(
        FileSize::from_bytes(1_099_511_627_776).to_string(),
        "1024.00 GB"
    );
    assert_eq!(
        FileSize::from_bytes(2_199_023_255_552).to_string(),
        "2048.00 GB"
    );
}

#[test]
fn test_unique_destination_no_conflict() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let dest_path = temp_dir.path().join("new_file.txt");

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("new_file.txt"))?;
    assert_eq!(result, dest_path);

    Ok(())
}

#[test]
fn test_unique_destination_with_conflicts() -> io::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create existing files
    fs::write(temp_dir.path().join("file.txt"), b"original")?;
    fs::write(temp_dir.path().join("file_1.txt"), b"copy1")?;
    fs::write(temp_dir.path().join("file_2.txt"), b"copy2")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file.txt"))?;

    assert_eq!(result, temp_dir.path().join("file_3.txt"));

    Ok(())
}

// Test unique_destination with no extension
#[test]
fn test_unique_destination_no_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("README"), b"data")?;
    fs::write(temp_dir.path().join("README_1"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("README"))?;
    assert_eq!(result.file_name().unwrap(), "README_2");

    Ok(())
}

// Test unique_destination with empty extension
#[test]
fn test_unique_destination_empty_extension() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file."), b"data")?;
    fs::write(temp_dir.path().join("file._1"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file."))?;
    assert_eq!(result.file_name().unwrap(), "file_1");

    Ok(())
}

// Test unique_destination with dotfile
#[test]
fn test_unique_destination_dotfile() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join(".config"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new(".config"))?;
    assert_eq!(result.file_name().unwrap(), ".config_1");

    Ok(())
}

// Test unique_destination with multiple dots
#[test]
fn test_unique_destination_multiple_dots() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("file.tar.gz"), b"data")?;

    let result = unique_destination(temp_dir.path(), std::ffi::OsStr::new("file.tar.gz"))?;
    assert_eq!(result.file_name().unwrap(), "file.tar_1.gz");

    Ok(())
}

// Test format_size edge cases
#[test]
fn test_format_size_edge_cases() {
    // Test exact boundaries
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
}
