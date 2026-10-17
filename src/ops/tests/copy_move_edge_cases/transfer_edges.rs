//! Purpose: Copy and move with very long paths, into the same folder, and of marked files.

use super::*;

/// A destination well past Windows' historical 260-character `MAX_PATH` must
/// either receive the file or say why. The one outcome a file manager may not
/// produce is a "Copied to …" with no file under it.
#[test]
#[cfg(not(coverage))]
fn test_handle_copy_move_with_very_long_paths() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");

    let long_path = long_destination(temp_dir.path());

    let source_file = temp_dir.path().join("source.txt");
    File::create(&source_file)
        .expect("test setup")
        .write_all(b"content")
        .expect("test setup");

    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = long_path.clone();
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");
    // By name, not by index: directories sort first, so index 1 is the nested
    // directory the destination lives *inside*, and copying that only ever
    // exercised the copy-into-itself guard — never a long path.
    assert!(app.left_panel.select_named("source.txt"));

    // Per-file failures are folded into the status summary, so the call itself
    // succeeds whichever way the filesystem went.
    handle_copy(&mut app).expect("handle_copy reports per-file errors in its status");

    let copied = long_path.join("source.txt");
    if copied.exists() {
        assert_eq!(fs::read_to_string(&copied)?, "content");
        assert!(
            app.status_text().contains("Copied to"),
            "a copy that landed must be reported as one: {}",
            app.status_text()
        );
    } else {
        assert!(
            app.status_text().contains("error"),
            "a copy that did not land must say so, not report success: {}",
            app.status_text()
        );
    }

    Ok(())
}

#[test]
fn test_handle_copy_to_same_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("file.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    // Select file.txt (skip ..)
    app.left_panel.select(Some(1));

    handle_copy(&mut app).expect("test setup");

    // Should create a unique name like file_1.txt
    assert!(temp_dir.path().join("file_1.txt").exists());
    assert!(app.status_text().contains("Copied to"));

    Ok(())
}

#[test]
fn test_handle_move_to_same_directory() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("move_me.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();
    app.left_panel.load_entries().expect("test setup");

    // Select move_me.txt
    app.left_panel.select(Some(1));

    handle_move(&mut app).expect("test setup");

    // Should be renamed to move_me_1.txt
    assert!(!file_path.exists());
    assert!(temp_dir.path().join("move_me_1.txt").exists());
    assert!(app.status_text().contains("Moved to"));

    Ok(())
}

#[test]
fn test_handle_copy_marked_to_different_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");
    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let f1 = left_dir.join("f1.txt");
    let f2 = left_dir.join("f2.txt");
    File::create(&f1).expect("test setup");
    File::create(&f2).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = left_dir;
    app.right_panel.path = right_dir.clone();
    app.left_panel.load_entries().expect("test setup");

    app.left_panel.marked.insert(FileName::from("f1.txt"));
    app.left_panel.marked.insert(FileName::from("f2.txt"));

    handle_copy(&mut app).expect("test setup");

    assert!(right_dir.join("f1.txt").exists());
    assert!(right_dir.join("f2.txt").exists());
    assert!(app.status_text().contains("Copied 2 item(s)"));

    Ok(())
}

#[cfg(not(coverage))]
fn long_destination(root: &Path) -> PathBuf {
    let mut long_path = root.to_path_buf();
    for i in 0..10 {
        long_path = long_path.join(format!("very_long_directory_name_{i}"));
        fs::create_dir(&long_path).expect("test setup");
    }
    assert!(
        long_path.to_string_lossy().len() > 260,
        "the destination has to clear MAX_PATH for this to be testing anything"
    );

    long_path
}
