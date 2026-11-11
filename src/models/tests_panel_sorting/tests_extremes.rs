//! Purpose: unit tests for extreme input: Unicode and very long names, the
//! bookmark slots' bounds, and a viewer over extreme content.

use super::*;

#[test]
fn test_panel_unicode_filenames() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");

    let unicode_names = vec![
        "файл.txt",
        "文件.doc",
        "αρχείο.pdf",
        "🎵music🎵.mp3",
        "file with spaces.txt",
    ];

    for name in &unicode_names {
        File::create(temp_dir.path().join(name)).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    assert_eq!(panel.entries.len(), unicode_names.len() + 1);

    for name in &unicode_names {
        let found = panel.entries.iter().any(|e| e.name == *name);
        assert!(found, "Unicode filename {name} not found");
    }

    Ok(())
}

#[test]
fn test_panel_very_long_filenames() -> io::Result<()> {
    #[cfg(not(windows))]
    {
        let temp_dir = TempDir::new().expect("test setup");
        let long_name = "a".repeat(255);
        let file_path = temp_dir.path().join(&long_name);
        File::create(&file_path).expect("test setup");
        let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        panel.load_entries().expect("test setup");

        let found = panel.entries.iter().any(|e| e.name == long_name);
        assert!(found);
    }
    #[cfg(windows)]
    {
        // Long filename test skipped on Windows
    }

    Ok(())
}

#[test]
fn test_app_bookmarks_boundary() -> io::Result<()> {
    let mut app = App::new().expect("test setup");

    assert_eq!(app.bookmarks.slots().len(), 10);

    for (i, bookmark) in app.bookmarks.slots().iter().enumerate() {
        assert!(bookmark.is_none(), "Bookmark {i} should be None");
    }

    app.bookmarks.set(
        BookmarkSlot::from_usize(0).expect("slot"),
        PathBuf::from("/test1"),
    );
    app.bookmarks.set(
        BookmarkSlot::from_usize(9).expect("slot"),
        PathBuf::from("/test2"),
    );

    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(0).expect("slot")),
        Some(Path::new("/test1"))
    );
    assert_eq!(
        app.bookmarks
            .get(BookmarkSlot::from_usize(9).expect("slot")),
        Some(Path::new("/test2"))
    );

    Ok(())
}

#[test]
fn test_viewer_with_extreme_content() {
    let viewer = Viewer::report("Large", vec!["x".repeat(1000); 10000]);

    assert_eq!(viewer.lines.len(), 10000);
    assert_eq!(viewer.lines[0].len(), 1000);

    let empty_viewer = Viewer::report("Empty", vec![]);

    assert_eq!(empty_viewer.lines.len(), 0);
}
