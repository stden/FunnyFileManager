//! Purpose: Real external-editor arguments and private-terminal dispatch of the
//! public `handle_edit` entrypoint.
use super::*;

#[test]
fn the_public_entrypoint_uses_the_private_terminal_mode() -> io::Result<()> {
    let dir = TempDir::new()?;
    fs::write(dir.path().join("notes.txt"), "text")?;
    let mut app = build_app(&dir);
    assert!(select_entry(&mut app.left_panel, "notes.txt"));
    app.live.terminal_mode = crate::models::TerminalMode::Headless;
    let _editor = EditorEnvGuard::set(&dir.path().join("missing-editor"));
    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let mut terminal = Terminal::new(WritableTestBackend::new(30, 10))?;
    super::super::handle_edit(&mut terminal, &mut app)?;
    assert!(app.status_text().starts_with("Failed to launch editor:"));
    assert_eq!(
        host.snapshot().state,
        crate::terminal::headless::ExternalState::Failed
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn dash_prefixed_relative_file_is_passed_as_a_file_argument() -> io::Result<()> {
    let dir = TempDir::new()?;
    let log = dir.path().join("argv");
    let script = dir.path().join("record-editor");
    crate::test_support::write_executable(
        &script,
        format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", log.display()).as_bytes(),
    )?;
    let status = super::super::run_editor(
        script.to_str().expect("UTF-8 fixture"),
        &["-w".to_owned()],
        Path::new("-notes.txt"),
    )?;
    assert!(status.success());
    assert_eq!(fs::read_to_string(log)?, "-w\n./-notes.txt\n");
    Ok(())
}
