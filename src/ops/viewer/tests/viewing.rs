//! Purpose: F3 on files: text, UTF-16 with a BOM, NUL bytes, and the view history.

use super::*;

#[test]
fn test_ops_view_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("file.txt");
    fs::write(&file, b"content")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("file.txt"));

    handle_view(&mut app)?;
    assert!(app.get_viewer().is_some());

    // Nonexistent file selection
    app.left_panel.select(None);
    handle_view(&mut app)?;

    // Directory selection: F3 measures its total size in the panel row without opening an overlay
    app.set_overlay(Overlay::None);
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;
    fs::write(subdir.join("nested.txt"), b"hello world")?;
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("subdir"));
    handle_view(&mut app)?;
    assert!(
        matches!(app.overlay, Overlay::None),
        "F3 on a directory updates its panel size in-place without opening a modal overlay"
    );
    let entry = app
        .left_panel
        .get_selected_entry()
        .expect("subdir selected");
    assert!(entry.directory_size.is_measured());
    assert_eq!(entry.size.bytes(), 11);

    Ok(())
}

#[test]
fn f3_on_dotdot_measures_all_directories_in_active_panel() -> io::Result<()> {
    let temp = TempDir::new()?;
    let alpha = temp.path().join("alpha");
    let beta = temp.path().join("beta");
    fs::create_dir(&alpha)?;
    fs::create_dir(&beta)?;
    fs::write(alpha.join("a.txt"), [b'a'; 100])?;
    fs::write(beta.join("b.txt"), [b'b'; 250])?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named(".."));

    handle_view(&mut app)?;

    assert!(matches!(app.overlay, Overlay::None));
    let alpha_entry = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name == "alpha")
        .expect("alpha");
    let beta_entry = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name == "beta")
        .expect("beta");
    assert!(alpha_entry.directory_size.is_measured());
    assert_eq!(alpha_entry.size.bytes(), 100);
    assert!(beta_entry.directory_size.is_measured());
    assert_eq!(beta_entry.size.bytes(), 250);
    Ok(())
}

#[test]
fn handle_view_records_the_file_behind_the_viewer() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("latin1.txt");
    fs::write(&file, b"caf\xe9\n")?;

    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "latin1.txt")
        .expect("listed");
    app.left_panel.select(Some(index));

    handle_view(&mut app)?;
    let viewer = app.get_viewer().expect("viewer open");
    assert_eq!(viewer.title(), "latin1.txt");
    assert_eq!(viewer.lines, vec!["caf\u{439}"]);
    let opened = viewer.viewed_file().expect("a file viewer knows its file");
    assert_eq!(opened.path, file);
    assert_eq!(opened.size.bytes(), 5);
    assert_eq!(opened.encoding, crate::encoding::TextEncoding::Windows1251);
    assert_eq!(app.status_text(), format!("Viewing {}", file.display()));
    Ok(())
}

/// UTF-16 text has a NUL in the high byte of every ASCII character. The
/// byte-order mark says so, and F3 must read the file rather than call it
/// binary — F4 already opens the same file.
#[test]
fn f3_opens_a_utf16_file_with_a_bom_instead_of_refusing_it_as_binary() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("utf16le.txt");
    fs::write(&file, b"\xff\xfeH\x00e\x00l\x00l\x00o\x00\n\x00")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, file.clone())?;

    let viewer = app.get_viewer().expect("F3 opens the viewer");
    assert_eq!(viewer.lines, vec!["Hello"]);
    assert_eq!(app.status, Status::Viewing(file.display().to_string()));
    Ok(())
}

/// The mark only excuses the NULs of text it names: a NUL-bearing file with
/// no byte-order mark is still refused.
#[test]
fn f3_still_refuses_a_nul_bearing_file_without_a_bom() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("blob.bin");
    fs::write(&file, b"MZ\x00\x01\x02")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, file.clone())?;

    assert!(app.get_viewer().is_none());
    assert_eq!(app.status, Status::BinaryFile(file.display().to_string()));
    Ok(())
}

#[test]
fn viewing_a_file_records_it_in_the_view_history() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("seen.txt");
    fs::write(&file, b"content")?;
    let mut app = App::new()?;
    app.left_panel.path = temp.path().to_path_buf();
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("seen.txt"));
    handle_view(&mut app)?;
    assert_eq!(app.view_history.last(), Some(file.as_path()));
    Ok(())
}

#[test]
fn viewing_expands_tabs_to_next_tab_stop() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("tabs.txt");
    fs::write(&file, "a\tb\nab\tc\n界\td\n")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, file)?;

    let viewer = app.get_viewer().expect("viewer open");
    assert_eq!(viewer.lines, vec!["a   b", "ab  c", "界  d"]);
    Ok(())
}
