use super::*;

// -----------------------------------------------------------------------
// Logging is opt-in, payload-free and owner-only
// -----------------------------------------------------------------------

#[test]
fn test_key_label_hides_typed_characters() {
    assert_eq!(key_label(KeyCode::Char('s')), "Char");
    assert_eq!(key_label(KeyCode::Char('3')), "Char");
    assert_eq!(key_label(KeyCode::F(3)), "F(3)");
    assert_eq!(key_label(KeyCode::Enter), "Enter");
}

/// The same rule as `key_label`, one level up: actions are logged before an
/// overlay consumes them, so an action carrying a typed character must not
/// print it. A derived `Debug` would write every password typed at the `:`
/// prompt into the log file, one keystroke per line.
#[test]
fn test_action_label_hides_typed_characters() {
    assert_eq!(
        action_label(Action::CommandLineChar('s')),
        "CommandLineChar(..)"
    );
    assert_eq!(action_label(Action::QuickSearch('s')), "QuickSearch(..)");
    assert_eq!(action_label(Action::Quit), "Quit");
    assert_eq!(action_label(Action::Copy), "Copy");
}

#[test]
fn test_logging_is_off_unless_opted_in() {
    if std::env::var_os("FFM_LOG").is_none() {
        assert!(
            log_dir().is_none(),
            "logging must stay off until FFM_LOG=1 is set"
        );
    }
}

#[test]
fn test_append_log_line_is_owner_only() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().join("ffm_actions.log");

    append_log_line(&path, "hello");
    append_log_line(&path, "again");

    let contents = fs::read_to_string(&path)?;
    assert!(contents.contains("] hello"), "{contents}");
    assert!(contents.contains("] again"), "{contents}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "log must not be readable by other users");
    }
    Ok(())
}

/// A log that cannot be opened is skipped silently — logging must never break
/// the file manager.
#[test]
fn test_append_log_line_ignores_open_failure() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let blocked = temp_dir.path().join("ffm_actions.log");
    fs::create_dir(&blocked)?;

    append_log_line(&blocked, "swallowed");

    assert!(blocked.is_dir());
    Ok(())
}
