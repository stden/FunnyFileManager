//! Purpose: unit tests for a successful `copy_dir_all`: files, empty folders, existing destinations and links.

use super::*;

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_copy_dir_all_mutation_killer() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("file.txt"), b"content").expect("test setup");

    // Test that function doesn't just return Ok(()) without doing anything
    copy_dir_all(&src, &dst).expect("test setup");

    // Verify the copy actually happened
    assert!(dst.exists());
    assert!(dst.join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(dst.join("file.txt")).expect("test setup"),
        "content"
    );

    Ok(())
}

#[test]
fn test_copy_dir_all_simple() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::write(src.join("file1.txt"), b"content1").expect("test setup");
    fs::write(src.join("file2.txt"), b"content2").expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.join("file1.txt").exists());
    assert!(dst.join("file2.txt").exists());

    let content1 = fs::read_to_string(dst.join("file1.txt")).expect("test setup");
    assert_eq!(content1, "content1");

    Ok(())
}

#[test]
fn test_copy_dir_all_empty_dir() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");

    copy_dir_all(&src, &dst).expect("test setup");

    assert!(dst.exists());
    assert!(dst.is_dir());
    assert_eq!(fs::read_dir(&dst).expect("test setup").count(), 0);

    Ok(())
}

#[test]
fn test_copy_dir_all_preserves_existing_destination_files() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    fs::create_dir(&dst).expect("test setup");
    File::create(src.join("new.txt"))?.write_all(b"new-data")?;
    File::create(dst.join("existing.txt"))?.write_all(b"keep-me")?;

    copy_dir_all(&src, &dst)?;

    assert_eq!(fs::read_to_string(dst.join("existing.txt"))?, "keep-me");
    assert_eq!(fs::read_to_string(dst.join("new.txt"))?, "new-data");
    Ok(())
}

#[test]
fn test_copy_dir_all_copies_zero_length_file() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    reset_forced_errors();

    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    File::create(src.join("empty.txt"))?;

    copy_dir_all(&src, &dst)?;

    let metadata = fs::metadata(dst.join("empty.txt"))?;
    assert!(metadata.is_file());
    assert_eq!(metadata.len(), 0);
    Ok(())
}

#[test]
fn test_copy_dir_all_with_symlinks() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let src = temp_dir.path().join("src");
    let dst = temp_dir.path().join("dst");

    fs::create_dir(&src).expect("test setup");
    let file_path = src.join("file.txt");
    File::create(&file_path)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_path = src.join("link.txt");
        symlink(&file_path, &link_path).expect("test setup");

        copy_dir_all(&src, &dst).expect("test setup");

        assert!(dst.join("file.txt").exists());
        // `exists()` follows links, so it passes even when the link was
        // dereferenced into a full copy — check the link itself.
        let copied_link = dst.join("link.txt");
        assert!(fs::symlink_metadata(&copied_link)
            .expect("test setup")
            .is_symlink());
        assert_eq!(fs::read_link(&copied_link).expect("test setup"), file_path);
    }

    #[cfg(not(unix))]
    {
        copy_dir_all(&src, &dst).expect("test setup");
        assert!(dst.join("file.txt").exists());
    }

    Ok(())
}
