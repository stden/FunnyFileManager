//! Unit tests: refresh and format (split out to keep files under 500 lines).
#![allow(unused_imports)]

use super::tests::*;
use crate::models::Bookmarks;
use crate::models::FileSize;
use crate::quicksearch::QuickSearch;
use crate::status::Status;

use super::*;

use crate::models::{App, Panel, PanelSide};

use std::ffi::OsStr;

use std::fs::File;

use std::io::Write;

use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;

#[test]
fn test_refresh_panels_maintains_selection() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");

    // Create files
    File::create(temp_dir.path().join("a.txt")).expect("test setup");
    File::create(temp_dir.path().join("b.txt")).expect("test setup");
    File::create(temp_dir.path().join("c.txt")).expect("test setup");

    let mut app = App {
        left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        active_panel: PanelSide::Left,
        status: Status::Cleared,
        overlay: crate::models::Overlay::None,
        bookmarks: Bookmarks::default(),
        lifecycle: crate::models::Lifecycle::default(),
        quick_search: QuickSearch::default(),
        view: crate::models::ViewMode::default(),
        history: crate::history::History::default(),
        console: crate::console::ConsoleLog::default(),
    };

    // Set selection to second file
    app.left_panel.select(Some(2));
    app.right_panel.select(Some(1));

    // Refresh panels
    refresh_panels(&mut app).expect("test setup");

    // Check selections are maintained
    assert!(app.left_panel.selected().is_some());
    assert!(app.right_panel.selected().is_some());

    Ok(())
}

#[test]
fn test_refresh_panels_sets_selection_when_none() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("only.txt")).expect("test setup");

    let mut app = App {
        left_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        right_panel: Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        active_panel: PanelSide::Left,
        status: Status::Cleared,
        overlay: crate::models::Overlay::None,
        bookmarks: Bookmarks::default(),
        lifecycle: crate::models::Lifecycle::default(),
        quick_search: QuickSearch::default(),
        view: crate::models::ViewMode::default(),
        history: crate::history::History::default(),
        console: crate::console::ConsoleLog::default(),
    };

    app.left_panel.select(None);
    app.right_panel.select(None);

    refresh_panels(&mut app).expect("test setup");

    assert!(app.left_panel.selected().is_some());
    assert!(app.right_panel.selected().is_some());

    Ok(())
}

#[test]
fn test_refresh_panels_keeps_paths_unchanged() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let left_path = temp_dir.path().join("left");
    let right_path = temp_dir.path().join("right");
    fs::create_dir(&left_path).expect("test setup");
    fs::create_dir(&right_path).expect("test setup");
    File::create(left_path.join("a.txt")).expect("test setup");
    File::create(right_path.join("b.txt")).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_path.clone()).expect("test setup");
    app.right_panel = Panel::new(right_path.clone()).expect("test setup");

    refresh_panels(&mut app).expect("test setup");

    assert_eq!(app.left_panel.path, left_path);
    assert_eq!(app.right_panel.path, right_path);
    Ok(())
}

#[test]
fn test_refresh_panels_loads_new_entries_in_both_panels() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let left_path = temp_dir.path().join("left");
    let right_path = temp_dir.path().join("right");
    fs::create_dir(&left_path).expect("test setup");
    fs::create_dir(&right_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(left_path.clone()).expect("test setup");
    app.right_panel = Panel::new(right_path.clone()).expect("test setup");

    File::create(left_path.join("left_new.txt")).expect("test setup");
    File::create(right_path.join("right_new.txt")).expect("test setup");

    refresh_panels(&mut app).expect("test setup");

    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "left_new.txt"));
    assert!(app
        .right_panel
        .entries
        .iter()
        .any(|e| e.name == "right_new.txt"));
    Ok(())
}

#[test]
fn test_format_size_edge_cases() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(
        FileSize::from_bytes(u64::MAX).to_string(),
        "17179869184.00 GB"
    );
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

#[test]
fn test_copy_dir_all_permission_errors() -> io::Result<()> {
    let _lock = copy_step_mutex().lock().expect("copy step mutex");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("test setup");
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src).expect("test setup");
        File::create(src.join("file.txt"))
            .expect("test setup")
            .write_all(b"content")
            .expect("test setup");

        fs::create_dir(&dst).expect("test setup");
        let mut perms = fs::metadata(&dst).expect("test setup").permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&dst, perms).expect("test setup");

        // Root bypasses Unix DAC permissions, so a 0o000 dst may still be
        // writable; only assert failure when perms are actually enforced.
        let perms_enforced = fs::read_dir(&dst).is_err();
        let result = copy_dir_all(&src, &dst);

        let mut perms = fs::metadata(&dst).expect("test setup").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dst, perms).expect("test setup");

        if perms_enforced {
            assert!(result.is_err());
        }
    }

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

#[test]
fn test_refresh_panels_with_deleted_directories() -> io::Result<()> {
    let _lock = super::refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let left_dir = temp_dir.path().join("left");
    let right_dir = temp_dir.path().join("right");

    fs::create_dir(&left_dir).expect("test setup");
    fs::create_dir(&right_dir).expect("test setup");

    let mut app = App {
        left_panel: Panel::new(left_dir.clone()).expect("test setup"),
        right_panel: Panel::new(right_dir.clone()).expect("test setup"),
        active_panel: PanelSide::Left,
        lifecycle: crate::models::Lifecycle::default(),
        overlay: crate::models::Overlay::None,
        status: Status::Cleared,
        bookmarks: Bookmarks::default(),
        quick_search: QuickSearch::default(),
        view: crate::models::ViewMode::default(),
        history: crate::history::History::default(),
        console: crate::console::ConsoleLog::default(),
    };

    fs::remove_dir(&left_dir).expect("test setup");
    fs::remove_dir(&right_dir).expect("test setup");

    let result = refresh_panels(&mut app);

    assert!(result.is_err() || app.left_panel.path != left_dir);

    Ok(())
}

#[test]
fn test_format_size_mutation_killer() {
    // Test that kills mutations that replace function with String::new() or "xyzzy"
    assert!(!FileSize::from_bytes(1000).to_string().is_empty());
    assert!(!FileSize::from_bytes(1000).to_string().contains("xyzzy"));

    // Test exact KB constant (1024, not 1024 + 0 or 1024 / 1)
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_ne!(FileSize::from_bytes(1025).to_string(), "1025 B"); // Should be KB

    // Test exact MB constant (1024 * 1024, not 1024 + 1024)
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_ne!(FileSize::from_bytes(1_048_577).to_string(), "1048577 B"); // Should be MB not bytes

    // Test exact GB constant
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );

    // Test >= vs < mutations
    assert_ne!(
        FileSize::from_bytes(1024).to_string(),
        FileSize::from_bytes(1023).to_string()
    ); // KB boundary
    assert_ne!(
        FileSize::from_bytes(1_048_576).to_string(),
        FileSize::from_bytes(1_048_575).to_string()
    ); // MB boundary
    assert_ne!(
        FileSize::from_bytes(1_073_741_824).to_string(),
        FileSize::from_bytes(1_073_741_823).to_string()
    ); // GB boundary
}

#[test]
fn test_format_size_precision() {
    assert_eq!(FileSize::from_bytes(1500).to_string(), "1.46 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1999).to_string(), "1.95 KB");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 + 512 * 1024).to_string(),
        "1.50 MB"
    );
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024 + 512 * 1024 * 1024).to_string(),
        "1.50 GB"
    );
}
