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

#[test]
fn system_profile_starts_and_finishes_without_touching_host_when_home_unset() -> io::Result<()> {
    let _lock = crate::test_support::env_lock();
    let vars = [
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ];
    let saved: Vec<_> = vars.iter().map(|&k| (k, std::env::var_os(k))).collect();
    for &k in &vars {
        std::env::remove_var(k);
    }
    let root = tempfile::TempDir::new()?;
    let mut session = app(root.path(), Profile::System)?;
    std::thread::sleep(std::time::Duration::from_millis(60));
    finish(&mut session, Profile::System);
    for (k, v) in saved {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
    assert_eq!(session.live.terminal_mode, TerminalMode::Headless);
    Ok(())
}
