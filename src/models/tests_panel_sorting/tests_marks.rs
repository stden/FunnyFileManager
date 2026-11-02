//! Purpose: unit tests for marking entries and the names an action takes.

use super::*;

#[test]
fn test_panel_marking_and_action_names() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("alpha.txt")).expect("test setup");
    File::create(temp_dir.path().join("beta.txt")).expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    select_named(&mut panel, "alpha.txt");

    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("alpha.txt"));
    assert_eq!(file_names(&panel), vec!["alpha.txt", "beta.txt"]);

    let mut action_names = panel.action_entry_names();
    action_names.sort();
    assert_eq!(action_names, vec!["alpha.txt"]);

    assert!(panel.toggle_mark_selected());
    assert_eq!(panel.marked.len(), 0);
    assert_eq!(panel.action_entry_names(), vec!["alpha.txt"]);

    select_named(&mut panel, "beta.txt");
    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("beta.txt"));

    fs::remove_file(temp_dir.path().join("beta.txt")).expect("test setup");
    panel.load_entries().expect("test setup");
    assert!(!panel.marked.contains("beta.txt"));

    panel.select(None);
    assert!(!panel.toggle_mark_selected());

    select_named(&mut panel, "..");
    assert!(!panel.toggle_mark_selected());
    assert_eq!(panel.action_entry_names().len(), 0);
    assert!(panel.selected_path().is_none());

    Ok(())
}

#[test]
fn test_panel_marking_operations_edge_cases() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    for i in 0..5 {
        File::create(temp_dir.path().join(format!("file{i}.txt"))).expect("test setup");
    }

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    panel.load_entries().expect("test setup");

    panel.select(Some(1));
    panel.toggle_mark_selected();
    panel.toggle_mark_selected();
    assert_eq!(panel.marked.len(), 0);

    for i in 1..=5 {
        panel.select(Some(i));
        panel.toggle_mark_selected();
    }
    assert_eq!(panel.marked.len(), 5);

    panel.clear_marks();
    assert_eq!(panel.marked.len(), 0);

    panel.select(Some(0));
    panel.toggle_mark_selected();
    assert_eq!(panel.marked.len(), 0);

    Ok(())
}

#[test]
fn test_toggle_mark_selected_no_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    // Clear selection
    panel.select(None);

    // Should return false when no selection
    assert!(!panel.toggle_mark_selected());
    assert_eq!(panel.marked.len(), 0);

    Ok(())
}

fn select_named(panel: &mut Panel, name: &str) {
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("named entry");
    panel.select(Some(index));
}
