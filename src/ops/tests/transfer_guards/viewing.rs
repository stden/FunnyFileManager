//! Purpose: F3 reports a binary file and decodes non-UTF-8 text lossily.

use super::*;

// -----------------------------------------------------------------------
// F3 on non-text files
// -----------------------------------------------------------------------

#[test]
fn handle_view_reports_binary_files_instead_of_failing() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("photo.bin"), [0xff, 0xd8, 0x00, 0x01])?;

    let mut app = app_with(temp_dir.path(), temp_dir.path(), "photo.bin")?;
    handle_view(&mut app).expect("a binary file is not a handler failure");

    assert!(matches!(app.overlay, Overlay::None));
    assert!(
        app.status_text().contains("Cannot view binary file"),
        "status was {:?}",
        app.status_text()
    );
    Ok(())
}

/// Text that merely is not valid UTF-8 (cp1251, latin-1, ...) still opens,
/// in the single-byte code page the editor falls back to.
#[test]
fn handle_view_decodes_non_utf8_text_lossily() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::write(temp_dir.path().join("cp1251.txt"), b"caf\xe9\nbar")?;

    let mut app = app_with(temp_dir.path(), temp_dir.path(), "cp1251.txt")?;
    handle_view(&mut app).expect("lossy decode must not fail the action");

    let viewer = app.get_viewer().expect("viewer must open");
    assert_eq!(
        viewer.lines,
        vec!["caf\u{439}".to_owned(), "bar".to_owned()]
    );
    Ok(())
}
