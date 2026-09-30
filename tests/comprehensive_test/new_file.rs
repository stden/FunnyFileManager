//! Purpose: New-file input: create, cancel, edit and name clashes.

use super::*;

// ==================== Overlay: New File Input (Shift+F4) ====================

#[test]
fn test_new_file_create() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT), // open new file dialog
        key(KeyCode::Char('n')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('w')),
        key(KeyCode::Char('.')),
        key(KeyCode::Char('t')),
        key(KeyCode::Char('x')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter), // create "new.txt"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("new.txt").exists());
    Ok(())
}

#[test]
fn test_new_file_empty_name() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Enter), // empty filename
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_new_file_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Char('x')),
        key(KeyCode::Esc), // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_new_file_backspace() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (app, mut terminal) = setup_app(&temp)?;

    let events = vec![
        key_mod(KeyCode::F(4), KeyModifiers::SHIFT),
        key(KeyCode::Char('a')),
        key(KeyCode::Char('b')),
        key(KeyCode::Backspace), // remove 'b'
        key(KeyCode::Enter),     // create "a"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("a").exists());
    Ok(())
}

#[test]
fn test_new_file_already_exists() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("exists.txt"), b"data").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::NewFileInput("exists.txt".to_owned().into()));

    let events = vec![
        key(KeyCode::Enter), // try to create existing file
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
