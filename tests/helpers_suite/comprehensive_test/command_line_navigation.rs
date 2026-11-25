//! Purpose: Command-line directory changes: cd, pwd and bookmarks.

use super::*;

// ==================== Overlay: Command Line ====================

#[test]
fn test_command_line_cd() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("target");
    fs::create_dir(&sub).unwrap();

    let (mut app, mut terminal) = setup_app(&temp)?;
    let target_str = sub.to_string_lossy().to_string();
    app.set_overlay(Overlay::CommandLine(format!("cd {target_str}").into()));

    let events = vec![
        key(KeyCode::Enter), // execute "cd /path/to/target"
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_line_pwd() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    let events = vec![
        key(KeyCode::Enter), // execute pwd
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_bookmark_set_and_go() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("bookmark set 0".into()));

    let events = vec![
        key(KeyCode::Enter), // set bookmark 0
        key(KeyCode::F(10)),
        // F10 now asks before quitting; Enter answers Yes.
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_bookmark_go_empty() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("bookmark go 5".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_cd_missing_arg() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("cd".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}

#[test]
fn test_command_cd_invalid_path() -> io::Result<()> {
    let temp = TempDir::new().unwrap();
    let (mut app, mut terminal) = setup_app(&temp)?;
    app.set_overlay(Overlay::CommandLine("cd /nonexistent_path_xyz_123".into()));

    let events = vec![
        key(KeyCode::Enter),
        key(KeyCode::F(10)),
        key(KeyCode::Enter),
    ];

    run_events(&mut terminal, app, events)
}
