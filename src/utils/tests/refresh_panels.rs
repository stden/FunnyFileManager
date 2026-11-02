//! Purpose: unit tests for `refresh_panels`: both panels re-read, selection and paths kept.

use super::*;

use crate::models::Panel;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn test_refresh_panels_mutation_killer() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir.path().to_path_buf();
    app.right_panel.path = temp_dir.path().to_path_buf();

    // Create initial state
    app.left_panel.load_entries().expect("test setup");
    app.right_panel.load_entries().expect("test setup");
    let initial_count = app.left_panel.entries.len();

    // Add a new file
    File::create(temp_dir.path().join("new_file.txt")).expect("test setup");

    // Test that refresh_panels actually does something
    refresh_panels(&mut app).expect("test setup");

    // Verify panels were actually refreshed
    assert!(app.left_panel.entries.len() > initial_count);
    assert!(app.right_panel.entries.len() > initial_count);
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "new_file.txt"));

    Ok(())
}

#[test]
fn test_refresh_panels() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");

    // Create initial files
    File::create(temp_dir.path().join("initial.txt")).expect("test setup");

    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
    );

    // Add new files after app creation
    File::create(temp_dir.path().join("new_file.txt")).expect("test setup");
    fs::create_dir(temp_dir.path().join("new_dir")).expect("test setup");

    // Refresh panels
    refresh_panels(&mut app).expect("test setup");

    // Check both panels contain the new entries
    for panel in [&app.left_panel, &app.right_panel] {
        let names: Vec<&str> = panel.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"new_file.txt") && names.contains(&"new_dir"));
    }

    Ok(())
}

#[test]
fn test_refresh_panels_maintains_selection() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");

    // Create files
    File::create(temp_dir.path().join("a.txt")).expect("test setup");
    File::create(temp_dir.path().join("b.txt")).expect("test setup");
    File::create(temp_dir.path().join("c.txt")).expect("test setup");

    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
    );

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
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("only.txt")).expect("test setup");

    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
        Panel::new(temp_dir.path().to_path_buf()).expect("test setup"),
    );

    app.left_panel.select(None);
    app.right_panel.select(None);

    refresh_panels(&mut app).expect("test setup");

    assert!(app.left_panel.selected().is_some());
    assert!(app.right_panel.selected().is_some());

    Ok(())
}

#[test]
fn test_refresh_panels_keeps_paths_unchanged() -> io::Result<()> {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
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
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
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
fn a_forced_refresh_failure_stays_on_the_thread_that_asked_for_it() {
    let _lock = refresh_panels_mutex().lock().expect("refresh mutex");
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().to_path_buf();
    set_refresh_panels_error(true);
    let other = std::thread::spawn(move || {
        let left = Panel::new(path.clone()).expect("test setup");
        let right = Panel::new(path).expect("test setup");
        let mut app = App::with_panels(left, right);
        refresh_panels(&mut app).expect("another thread is not forced to fail");
    });
    let joined = other.join();
    set_refresh_panels_error(false);
    joined.expect("worker thread");
}
