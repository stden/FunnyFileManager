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

    // Directory selection
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir)?;
    app.left_panel.load_entries()?;
    assert!(app.left_panel.select_named("subdir"));
    handle_view(&mut app)?;
    assert!(
        matches!(app.overlay, Overlay::Information(_)),
        "F3 on a directory measures it, as Far does"
    );

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
