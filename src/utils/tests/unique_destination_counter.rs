//! Purpose: unit tests for how the `unique_destination` counter advances and when it gives up.

use super::*;

use std::ffi::OsStr;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn test_unique_destination_sparse_conflict_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    File::create(base.join("report.txt")).unwrap();
    File::create(base.join("report_1.txt")).unwrap();
    File::create(base.join("report_3.txt")).unwrap();

    let dest = unique_destination(base, OsStr::new("report.txt")).unwrap();
    assert_eq!(dest, base.join("report_2.txt"));

    for i in [2, 4, 5, 6, 8] {
        File::create(base.join(format!("report_{i}.txt"))).unwrap();
    }
    let dest_after_four = unique_destination(base, OsStr::new("report.txt")).unwrap();
    assert_eq!(dest_after_four, base.join("report_7.txt"));
}

#[test]
fn test_unique_destination_mutation_killer() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    // Test that kills "replace with Default::default()" mutation
    let result = unique_destination(temp_dir.path(), OsStr::new("test.txt"))?;
    assert!(!result.as_os_str().is_empty());
    assert!(result.starts_with(temp_dir.path()));

    // Test match guard mutations for extension handling
    let file_with_ext = temp_dir.path().join("file.txt");
    File::create(&file_with_ext).expect("test setup");
    let unique = unique_destination(temp_dir.path(), OsStr::new("file.txt"))?;
    assert!(unique.to_string_lossy().contains("file_1.txt"));

    // Test empty extension branch
    let file_no_ext = temp_dir.path().join("file");
    File::create(&file_no_ext).expect("test setup");
    let unique = unique_destination(temp_dir.path(), OsStr::new("file"))?;
    assert!(unique.to_string_lossy().contains("file_1"));
    assert!(!unique.to_string_lossy().ends_with('.')); // No trailing dot

    // Test counter increment mutations (+= vs -= vs *=)
    File::create(temp_dir.path().join("count.txt")).expect("test setup");
    for i in 1..5 {
        File::create(temp_dir.path().join(format!("count_{i}.txt"))).expect("test setup");
    }
    let unique = unique_destination(temp_dir.path(), OsStr::new("count.txt"))?;
    // Should be count_5.txt, not count_-1.txt or count_0.txt
    assert!(unique.to_string_lossy().contains("count_5.txt"));
    assert!(!unique.to_string_lossy().contains("count_-"));
    assert!(!unique.to_string_lossy().contains("count_0"));

    Ok(())
}

#[test]
fn test_unique_destination_errors_after_max_attempts() {
    let temp_dir = TempDir::new().expect("test setup");
    let base = temp_dir.path();

    File::create(base.join("taken.txt")).expect("test setup");
    for i in 1..=9999 {
        File::create(base.join(format!("taken_{i}.txt"))).expect("test setup");
    }

    let err = unique_destination(base, OsStr::new("taken.txt")).expect_err("should exhaust names");
    assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    assert!(err.to_string().contains("could not find unique name"));
}
