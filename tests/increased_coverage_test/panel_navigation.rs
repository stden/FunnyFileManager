//! Purpose: Enter on a file, moving with no selection and the selection after a reload.

use super::*;

#[test]
fn test_panel_enter_file_is_noop() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("file.txt"), b"x")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let original_path = panel.path.clone();
    let file_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "file.txt")
        .expect("file entry");
    panel.select(Some(file_index));

    panel.enter()?;

    assert_eq!(panel.path, original_path);
    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("file.txt")
    );
    Ok(())
}

#[test]
fn test_panel_next_previous_with_none_selection() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("one.txt"), b"1")?;
    fs::write(temp.path().join("two.txt"), b"2")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.select(None);

    panel.next();
    assert_eq!(panel.selected(), Some(0));

    panel.select(None);
    panel.previous();
    assert_eq!(panel.selected(), Some(0));

    Ok(())
}

#[test]
fn test_panel_load_entries_retains_selected_name_after_reload() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("alpha.txt"), b"a")?;
    fs::write(temp.path().join("beta.txt"), b"b")?;

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let beta_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "beta.txt")
        .expect("beta entry");
    panel.select(Some(beta_index));

    fs::write(temp.path().join("gamma.txt"), b"g")?;
    panel.load_entries()?;

    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("beta.txt")
    );
    Ok(())
}
