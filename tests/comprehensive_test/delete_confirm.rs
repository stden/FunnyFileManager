//! Purpose: Delete confirmation: answers, marked files, directories and `..`.

use super::*;

// ==================== Overlay: Delete Confirmation ====================

#[test]
fn test_delete_confirm_yes() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("doomed.txt"), b"bye").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "doomed.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete confirmation
        key(KeyCode::Char('y')), // confirm
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("doomed.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_no() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("safe.txt"), b"keep").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "safe.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete confirmation
        key(KeyCode::Char('n')), // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("safe.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_escape() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("safe.txt"), b"keep").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "safe.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)), // trigger delete confirmation
        key(KeyCode::Esc),  // cancel
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(temp.path().join("safe.txt").exists());
    Ok(())
}

#[test]
fn test_delete_confirm_with_enter() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("gone.txt"), b"data").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "gone.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),  // trigger delete
        key(KeyCode::Enter), // confirm with Enter
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("gone.txt").exists());
    Ok(())
}

#[test]
fn test_delete_marked_files() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    fs::write(temp.path().join("c.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.marked.insert(FileName::from("a.txt"));
    app.left_panel.marked.insert(FileName::from("b.txt"));

    let events = vec![
        key(KeyCode::F(8)),      // trigger delete for marked files
        key(KeyCode::Char('Y')), // confirm with uppercase Y
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!temp.path().join("a.txt").exists());
    assert!(!temp.path().join("b.txt").exists());
    assert!(temp.path().join("c.txt").exists()); // not marked
    Ok(())
}

#[test]
fn test_delete_directory() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("deleteme");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("inside.txt"), b"").unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "deleteme")
        .unwrap();
    app.left_panel.select(Some(idx));

    let events = vec![
        key(KeyCode::F(8)),
        key(KeyCode::Char('y')),
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)?;
    assert!(!sub.exists());
    Ok(())
}

#[test]
fn test_delete_dotdot_does_nothing() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.left_panel.select(Some(0)); // ".."

    let events = vec![
        key(KeyCode::F(8)), // should do nothing for ".."
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
