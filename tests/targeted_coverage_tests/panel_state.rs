//! Purpose: Panel and app state: sort cycling, panel swap and mark-and-advance.

use super::*;

// ──────────────────────────────────────────────
// Model edge cases
// ──────────────────────────────────────────────

#[test]
fn test_panel_cycle_sort_mode() -> io::Result<()> {
    use funny_file_manager::models::SortMode;

    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"short")?;
    File::create(temp.path().join("b.rs"))?.write_all(b"longer content")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    assert!(matches!(app.left_panel.sort_mode, SortMode::Name));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Extension));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Size));
    app.left_panel.cycle_sort_mode();
    assert!(matches!(app.left_panel.sort_mode, SortMode::Name));

    Ok(())
}

#[test]
fn test_app_swap_panels() -> io::Result<()> {
    let temp = TempDir::new()?;
    let dir_a = temp.path().join("aa");
    let dir_b = temp.path().join("bb");
    fs::create_dir(&dir_a)?;
    fs::create_dir(&dir_b)?;

    let mut app = App::new()?;
    app.left_panel.path = dir_a.clone();
    app.right_panel.path = dir_b.clone();

    app.swap_panels();
    assert_eq!(app.left_panel.path, dir_b);
    assert_eq!(app.right_panel.path, dir_a);

    Ok(())
}

#[test]
fn test_panel_toggle_mark_and_advance() -> io::Result<()> {
    let temp = TempDir::new()?;
    File::create(temp.path().join("a.txt"))?.write_all(b"a")?;
    File::create(temp.path().join("b.txt"))?.write_all(b"b")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;

    // Select first non-".." entry
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "a.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }

    app.left_panel.toggle_mark_selected();
    assert!(app.left_panel.marked.contains("a.txt"));

    // Toggle again to unmark
    // Re-select a.txt since toggle_mark_selected may have advanced
    for (i, entry) in app.left_panel.entries.iter().enumerate() {
        if entry.name == "a.txt" {
            app.left_panel.select(Some(i));
            break;
        }
    }
    app.left_panel.toggle_mark_selected();
    assert!(!app.left_panel.marked.contains("a.txt"));

    Ok(())
}
