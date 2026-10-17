//! Purpose: Copying and pasting a Unicode selection: Ctrl+C, Ctrl+V and terminal paste.

use super::*;

#[test]
fn ctrl_c_copies_the_visible_unicode_selection_without_editing_or_quitting() -> io::Result<()> {
    let directory = TempDir::new()?;
    let file = directory.path().join(".env");
    fs::write(&file, ORIGINAL)?;
    let clipboard = Clipboard::default();
    let mut steps = open_and_select();
    steps.extend([
        Step::Key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Step::Inspect(|buffer| {
            assert_selection(buffer);
            assert!(row(buffer, 0).contains("Selection copied"));
        }),
    ]);
    steps.extend(finish());

    run(&directory, &clipboard, steps)?;

    assert_eq!(clipboard.0.borrow().copied, [COPIED]);
    assert_eq!(fs::read_to_string(file)?, ORIGINAL);
    Ok(())
}

#[test]
fn ctrl_v_replaces_the_selection_with_one_clipboard_read() -> io::Result<()> {
    let directory = TempDir::new()?;
    let file = directory.path().join(".env");
    fs::write(&file, ORIGINAL)?;
    let clipboard = Clipboard::default();
    let mut steps = open_and_select();
    steps.extend([
        Step::Clipboard(REPLACEMENT),
        Step::Key(KeyCode::Char('v'), KeyModifiers::CONTROL),
        Step::Inspect(assert_pasted_buffer),
    ]);
    steps.extend(finish());
    run(&directory, &clipboard, steps)?;
    assert_eq!(fs::read_to_string(file)?, SAVED);
    assert_eq!(clipboard.0.borrow().reads, 1);
    Ok(())
}

#[test]
fn terminal_paste_replaces_the_selection_as_a_complete_text_block() -> io::Result<()> {
    let directory = TempDir::new()?;
    let file = directory.path().join(".env");
    fs::write(&file, ORIGINAL)?;
    let clipboard = Clipboard::default();
    let mut steps = open_and_select();
    steps.extend([
        Step::Paste(REPLACEMENT.to_owned()),
        Step::Inspect(assert_pasted_buffer),
    ]);
    steps.extend(finish());
    run(&directory, &clipboard, steps)?;
    assert_eq!(fs::read_to_string(file)?, SAVED);
    assert_eq!(clipboard.0.borrow().reads, 0);
    Ok(())
}
