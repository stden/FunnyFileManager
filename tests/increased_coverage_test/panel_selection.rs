//! Purpose: The selected path, action targets and marks, including `..` and empty panels.

use super::*;

#[test]
fn test_panel_selected_path_and_action_names() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let alpha_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "alpha.txt")
        .expect("alpha.txt entry");
    panel.select(Some(alpha_index));

    assert_eq!(panel.selected_path(), Some(temp.path().join("alpha.txt")));
    assert_eq!(panel.action_entry_names(), vec!["alpha.txt".to_owned()]);

    panel.marked.insert(FileName::from("beta.txt"));
    assert_eq!(panel.action_entry_names(), vec!["beta.txt".to_owned()]);

    Ok(())
}

#[test]
fn test_panel_selected_path_for_parent_entry_is_none() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut panel = Panel::new(subdir)?;
    let parent_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("parent entry");
    panel.select(Some(parent_index));

    assert_eq!(panel.selected_path(), None);
    assert!(panel.action_entry_names().is_empty());

    Ok(())
}

#[test]
fn test_panel_ensure_selection_and_clear_marks() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);
    panel.ensure_selection();
    assert_eq!(panel.selected(), Some(0));

    panel.marked.insert(FileName::from("file.txt"));
    panel.clear_marks();
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_toggle_mark_selected_ignores_parent_entry() -> io::Result<()> {
    let temp = TempDir::new()?;
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;

    let mut panel = Panel::new(subdir)?;
    let parent_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "..")
        .expect("parent entry");
    panel.select(Some(parent_index));

    assert!(!panel.toggle_mark_selected());
    assert!(panel.marked.is_empty());

    Ok(())
}

#[test]
fn test_panel_action_methods_on_empty_panel() {
    let mut panel = Panel::from_entries(PathBuf::new(), Vec::new());

    assert_eq!(panel.selected_path(), None);
    assert_eq!(panel.action_entry_names(), Vec::<String>::new());
    assert!(!panel.find_first_matching("anything"));
    assert!(!panel.find_next_matching("anything"));
    assert!(!panel.toggle_mark_selected());

    panel.next();
    panel.previous();
    panel.ensure_selection();
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_panel_toggle_mark_selected_marks_and_unmarks_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file entry");
    panel.select(Some(file_index));

    assert!(panel.toggle_mark_selected());
    assert!(panel.marked.contains("file.txt"));

    assert!(panel.toggle_mark_selected());
    assert!(!panel.marked.contains("file.txt"));

    Ok(())
}
