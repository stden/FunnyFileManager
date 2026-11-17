//! Purpose: Utilities: unique destinations, recursive copy, panel refresh, sizes and the test
//! backend.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// unique_destination edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_unique_destination_no_conflict() -> io::Result<()> {
    let temp = TempDir::new()?;
    let result = unique_destination(temp.path(), OsStr::new("fresh.txt"))?;
    assert_eq!(result.file_name().unwrap(), "fresh.txt");
    Ok(())
}

#[test]
fn test_unique_destination_with_conflict() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("taken.txt"), "x")?;
    let result = unique_destination(temp.path(), OsStr::new("taken.txt"))?;
    assert_eq!(result.file_name().unwrap(), "taken_1.txt");
    Ok(())
}

#[test]
fn test_unique_destination_multiple_conflicts() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("dup.txt"), "0")?;
    fs::write(temp.path().join("dup_1.txt"), "1")?;
    fs::write(temp.path().join("dup_2.txt"), "2")?;
    let result = unique_destination(temp.path(), OsStr::new("dup.txt"))?;
    assert_eq!(result.file_name().unwrap(), "dup_3.txt");
    Ok(())
}

#[test]
fn test_unique_destination_no_extension() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("README"), "data")?;
    let result = unique_destination(temp.path(), OsStr::new("README"))?;
    let name = result.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("README_"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// copy_dir_all edge case
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_copy_dir_all_nested() -> io::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("src");
    fs::create_dir_all(src.join("sub1").join("sub2"))?;
    fs::write(src.join("root.txt"), "r")?;
    fs::write(src.join("sub1").join("mid.txt"), "m")?;
    fs::write(src.join("sub1").join("sub2").join("deep.txt"), "d")?;
    let dst = temp.path().join("dst");
    copy_dir_all(&src, &dst)?;
    assert!(dst.join("root.txt").exists());
    assert!(dst.join("sub1").join("mid.txt").exists());
    assert!(dst.join("sub1").join("sub2").join("deep.txt").exists());
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// refresh_panels
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_refresh_panels_reloads() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = setup_app(&temp)?;
    // Create file after panel was loaded
    fs::write(temp.path().join("new_after_load.txt"), "new")?;
    refresh_panels(&mut app)?;
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_after_load.txt"));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// format_size boundary values
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_format_size_boundaries() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1025).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(
        FileSize::from_bytes(u64::MAX).to_string(),
        // 2^64 - 1 bytes is a hair under 2^34 GiB, which rounds up.
        "17179869184.00 GB"
    );
}

// ═══════════════════════════════════════════════════════════════════
// WritableTestBackend coverage
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_writable_test_backend_basic() -> io::Result<()> {
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|_f| {})?;
    Ok(())
}

#[test]
fn test_writable_test_backend_write() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    write!(backend, "hello")?;
    Ok(())
}
