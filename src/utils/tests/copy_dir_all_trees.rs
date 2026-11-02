//! Purpose: unit tests for `copy_dir_all` over nested, deep and unusually named trees.

use super::*;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_copy_dir_all_nested() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::create_dir(src.join("subdir1")).expect("test setup");
    fs::create_dir(src.join("subdir1").join("subsubdir")).expect("test setup");
    fs::create_dir(src.join("subdir2")).expect("test setup");

    fs::write(src.join("root.txt"), b"root").expect("test setup");
    fs::write(src.join("subdir1").join("file1.txt"), b"sub1").expect("test setup");
    fs::write(
        src.join("subdir1").join("subsubdir").join("deep.txt"),
        b"deep",
    )
    .expect("test setup");
    fs::write(src.join("subdir2").join("file2.txt"), b"sub2").expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.join("root.txt").exists());
    assert!(dst.join("subdir1").join("file1.txt").exists());
    assert!(dst
        .join("subdir1")
        .join("subsubdir")
        .join("deep.txt")
        .exists());
    assert!(dst.join("subdir2").join("file2.txt").exists());

    Ok(())
}

#[test]
fn test_copy_dir_all_deeply_nested() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    let mut current = src.clone();
    for i in 0..10 {
        current = current.join(format!("level{i}"));
        fs::create_dir_all(&current).expect("test setup");
        File::create(current.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");
    }

    copy_dir_all(&src, &dst).expect("test setup");

    let mut check = dst;
    for i in 0..10 {
        check = check.join(format!("level{i}"));
        assert!(check.exists());
        assert!(check.join("file.txt").exists());
    }

    Ok(())
}

#[test]
fn test_copy_dir_all_with_special_filenames() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");

    let special_names = vec![
        "file with spaces.txt",
        "file[brackets].txt",
        "file(parens).txt",
        "file{braces}.txt",
        ".hidden",
        "ñoño.txt",
        "файл.txt",
    ];

    for name in &special_names {
        File::create(src.join(name))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");
    }

    copy_dir_all(&src, &dst).expect("test setup");

    for name in &special_names {
        assert!(dst.join(name).exists(), "File {name} not copied");
    }

    Ok(())
}
