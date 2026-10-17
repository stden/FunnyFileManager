//! Purpose: Size formatting, unique destinations and recursive directory copy.

use super::*;

// ---------------------------------------------------------------------------
// format_size edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_format_size_zero() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
}

#[test]
fn test_format_size_exact_boundaries() {
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
}

#[test]
fn test_format_size_large() {
    let size = FileSize::from_bytes(u64::MAX).to_string();
    assert!(size.contains("GB") || size.contains("EB"));
}

// ---------------------------------------------------------------------------
// unique_destination edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_unique_destination_no_conflict() {
    let temp = TempDir::new().unwrap();
    let result = unique_destination(temp.path(), std::ffi::OsStr::new("new.txt")).unwrap();
    assert_eq!(result, temp.path().join("new.txt"));
}

#[test]
fn test_unique_destination_with_conflict() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("file.txt")).unwrap();
    assert_eq!(result, temp.path().join("file_1.txt"));
}

#[test]
fn test_unique_destination_multiple_conflicts() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("f.txt"), "x").unwrap();
    fs::write(temp.path().join("f_1.txt"), "x").unwrap();
    fs::write(temp.path().join("f_2.txt"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("f.txt")).unwrap();
    assert_eq!(result, temp.path().join("f_3.txt"));
}

#[test]
fn test_unique_destination_no_extension() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Makefile"), "x").unwrap();

    let result = unique_destination(temp.path(), std::ffi::OsStr::new("Makefile")).unwrap();
    assert_eq!(result, temp.path().join("Makefile_1"));
}

// ---------------------------------------------------------------------------
// copy_dir_all
// ---------------------------------------------------------------------------

#[test]
fn test_copy_dir_all_nested() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(src.join("sub1/sub2")).unwrap();
    fs::write(src.join("root.txt"), "root").unwrap();
    fs::write(src.join("sub1/a.txt"), "a").unwrap();
    fs::write(src.join("sub1/sub2/b.txt"), "b").unwrap();

    copy_dir_all(&src, &dst).unwrap();

    assert!(dst.join("root.txt").exists());
    assert!(dst.join("sub1/a.txt").exists());
    assert!(dst.join("sub1/sub2/b.txt").exists());
    assert_eq!(fs::read_to_string(dst.join("root.txt")).unwrap(), "root");
}

#[test]
fn test_copy_dir_all_src_not_exist() {
    let temp = TempDir::new().unwrap();
    let result = copy_dir_all(&temp.path().join("nope"), &temp.path().join("dst"));
    assert!(result.is_err());
}
