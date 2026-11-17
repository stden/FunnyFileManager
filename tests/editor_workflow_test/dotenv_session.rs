//! Purpose: A whole .env editing session, and a failed paste that keeps the selection.

use super::*;

#[test]
fn text_ui_edits_a_real_dotenv_file_with_selection_clipboard_resize_and_save() -> io::Result<()> {
    let directory = TempDir::new()?;
    let file = directory.path().join(".env");
    fs::write(&file, ORIGINAL)?;
    let clipboard = Clipboard::default();
    let mut steps = open_and_select();
    steps.extend([
        Step::Key(KeyCode::Insert, KeyModifiers::CONTROL),
        Step::Clipboard(REPLACEMENT),
        Step::Resize(Size::new(18, 6)),
        Step::Inspect(|buffer| {
            assert_eq!(buffer.area.width, 18);
            assert_eq!(buffer.area.height, 6);
            assert_selection(buffer);
        }),
        shifted(KeyCode::Insert),
        Step::Inspect(assert_pasted_buffer),
        Step::Resize(Size::new(90, 20)),
    ]);
    steps.extend(finish());
    run(&directory, &clipboard, steps)?;
    assert_eq!(
        fs::read_to_string(file)?,
        SAVED,
        "F2 must preserve CRLF and unchanged trailing lines"
    );
    assert_eq!(clipboard.0.borrow().copied, [COPIED]);
    assert_eq!(clipboard.0.borrow().reads, 1);
    Ok(())
}

#[test]
fn text_ui_failed_paste_keeps_the_visible_selection_and_saved_file() -> io::Result<()> {
    let directory = TempDir::new()?;
    let file = directory.path().join(".env");
    fs::write(&file, ORIGINAL)?;
    let clipboard = Clipboard::default();
    clipboard.0.borrow_mut().fail_read = true;
    let mut steps = open_and_select();
    steps.extend([
        Step::Resize(Size::new(18, 6)),
        shifted(KeyCode::Insert),
        Step::Inspect(|buffer| {
            assert_eq!(buffer.area.width, 18);
            assert_selection(buffer);
            assert!(
                row(buffer, 0).contains("Paste failed"),
                "failure must be visible inside editor"
            );
        }),
        Step::Resize(Size::new(90, 20)),
        Step::Inspect(assert_failed_paste_header),
    ]);
    steps.extend(finish());
    run(&directory, &clipboard, steps)?;
    assert_eq!(fs::read_to_string(file)?, ORIGINAL);
    assert_eq!(clipboard.0.borrow().reads, 1);
    assert_eq!(clipboard.0.borrow().copied.len(), 0);
    Ok(())
}

fn assert_failed_paste_header(buffer: &Buffer) {
    assert_selection(buffer);
    let header = row(buffer, 0);
    assert!(header.contains("Paste failed"));
    assert!(header.contains(" Ln ") && header.contains(" Col "));
}
