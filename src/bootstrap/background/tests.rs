//! Purpose: Closing an isolated session clears its own pending clipboard secret.

use super::*;

#[test]
fn isolated_finish_disarms_and_clears_only_the_session_clipboard() -> io::Result<()> {
    let root = tempfile::TempDir::new()?;
    let mut session = app(root.path(), Profile::Isolated)?;
    assert_eq!(session.live.terminal_mode, TerminalMode::Headless);
    session
        .clipboard
        .backend_mut()
        .unwrap()
        .set_native("session secret")
        .unwrap();
    session.clipboard.arm(std::time::Instant::now());
    finish(&mut session, Profile::Isolated);
    assert!(!session.clipboard.is_armed());
    assert_eq!(
        session
            .clipboard
            .backend_mut()
            .unwrap()
            .get_native()
            .unwrap(),
        ""
    );
    finish(&mut session, Profile::Isolated);
    assert!(!session.clipboard.is_armed());
    Ok(())
}

#[test]
fn finishing_an_armed_slot_without_a_backend_still_disarms_it() -> io::Result<()> {
    let root = tempfile::TempDir::new()?;
    let mut session = app(root.path(), Profile::Isolated)?;
    session.clipboard = crate::models::ClipboardSlot::default();
    session.clipboard.arm(std::time::Instant::now());
    finish(&mut session, Profile::Isolated);
    assert!(!session.clipboard.is_armed());
    assert!(session.clipboard.backend_mut().is_none());
    Ok(())
}
