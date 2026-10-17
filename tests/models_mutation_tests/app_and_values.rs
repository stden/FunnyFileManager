//! Purpose: App panel getters and switching, the viewer and directory entries.

use super::*;

#[test]
fn test_app_panel_getters_mutations() -> io::Result<()> {
    let mut app = App::new()?;

    // Test get_active_panel with Left
    app.active_panel = PanelSide::Left;
    {
        let active = app.get_active_panel();
        active.sort_mode = SortMode::Size;
    }
    assert_eq!(app.left_panel.sort_mode, SortMode::Size);
    assert_eq!(app.right_panel.sort_mode, SortMode::Name);

    // Test get_active_panel with Right
    app.active_panel = PanelSide::Right;
    {
        let active = app.get_active_panel();
        active.sort_mode = SortMode::Extension;
    }
    assert_eq!(app.right_panel.sort_mode, SortMode::Extension);

    // Test get_inactive_panel with Left active
    app.active_panel = PanelSide::Left;
    {
        let inactive = app.get_inactive_panel();
        inactive.sort_mode = SortMode::Size;
    }
    assert_eq!(app.right_panel.sort_mode, SortMode::Size);

    // Test get_inactive_panel with Right active
    app.active_panel = PanelSide::Right;
    {
        let inactive = app.get_inactive_panel();
        inactive.sort_mode = SortMode::Name;
    }
    assert_eq!(app.left_panel.sort_mode, SortMode::Name);

    Ok(())
}

#[test]
fn test_app_switch_panel_mutations() -> io::Result<()> {
    let mut app = App::new()?;

    // Test switching from Left to Right
    app.active_panel = PanelSide::Left;
    app.switch_panel();
    assert!(matches!(app.active_panel, PanelSide::Right));

    // Test switching from Right to Left
    app.switch_panel();
    assert!(matches!(app.active_panel, PanelSide::Left));

    Ok(())
}

#[test]
fn test_viewer_mutations() {
    // Test creation
    let viewer = Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("test.txt", vec!["Line 1".to_owned(), "Line 2".to_owned()])
    };

    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 2);
    assert_eq!(viewer.scroll.lines(), 5);

    // Test default
    let default_viewer = Viewer::default();
    assert!(default_viewer.title().is_empty());
    assert!(default_viewer.lines.is_empty());
    assert_eq!(default_viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_dir_entry_mutations() {
    // Test creation
    let entry = DirEntry {
        name: "test.txt".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    assert_eq!(entry.name, "test.txt");
    assert!(!entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(1024));

    // Test clone
    let cloned = entry.clone();
    assert_eq!(cloned.name, entry.name);
    assert_eq!(cloned.kind.is_dir(), entry.kind.is_dir());
    assert_eq!(cloned.size, entry.size);
}
