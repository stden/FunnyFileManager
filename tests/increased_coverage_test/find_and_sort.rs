//! Purpose: Quick-find matching and sort modes.

use super::*;

#[test]
fn test_sort_mode_labels_and_cycle() {
    assert_eq!(SortMode::Name.label(), "Name");
    assert_eq!(SortMode::Extension.label(), "Ext");
    assert_eq!(SortMode::Size.label(), "Size");

    assert_eq!(SortMode::Name.next(), SortMode::Extension);
    assert_eq!(SortMode::Extension.next(), SortMode::Size);
    assert_eq!(SortMode::Size.next(), SortMode::Name);
}

#[test]
fn test_panel_find_matching_variants() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("alphabet.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    assert!(panel.find_first_matching("alp"));
    let first = panel
        .get_selected_entry()
        .expect("selected first match")
        .name
        .clone();
    assert!(first.starts_with("alp"));

    assert!(panel.find_next_matching("alp"));
    let second = panel
        .get_selected_entry()
        .expect("selected next match")
        .name
        .clone();
    assert!(second.starts_with("alp"));

    assert!(!panel.find_first_matching(""));
    assert!(!panel.find_next_matching("zzz"));

    Ok(())
}

#[test]
fn test_panel_cycle_sort_mode_changes_reload_order() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("dir"))?;
    // Name sort: a.txt, b.log; Extension sort: b.log, a.txt (.log < .txt)
    fs::write(temp.path().join("a.txt"), b"1")?;
    fs::write(temp.path().join("b.log"), b"12345")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.sort_mode = SortMode::Name;
    panel.load_entries()?;
    let names_before: Vec<String> = panel
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();

    panel.cycle_sort_mode();
    panel.load_entries()?;
    let names_after: Vec<String> = panel
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();

    assert_ne!(panel.sort_mode, SortMode::Name);
    assert_ne!(names_before, names_after);
    Ok(())
}

#[test]
fn test_panel_find_next_matching_wraps_back_to_first_match() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("alpine.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let alpine_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "alpine.txt")
        .expect("alpine entry");
    panel.select(Some(alpine_index));

    assert!(panel.find_next_matching("alp"));
    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("alpha.txt")
    );

    Ok(())
}
