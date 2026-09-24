//! Unlock-path tests: a failed unlock stays locked, says why without echoing
//! the typed password, and clears what was typed.

use super::*;

#[test]
fn wrong_password_stays_locked_records_the_error_and_clears_what_was_typed() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, "definitely not it", now);

    assert!(!session.unlock(now));

    assert!(session.vault().is_none(), "must not unlock");
    let prompt = session.prompt().expect("still locked");
    assert_eq!(prompt.error(), Some("Wrong password or key file"));
    // A wrong secret is still a secret: it must not be left sitting in the
    // prompt waiting for the next attempt.
    assert!(prompt.is_empty());
    assert_eq!(prompt.typed_len(), 0);
    assert_eq!(prompt.masked(), "");
}

#[test]
fn unlock_error_never_echoes_the_typed_password() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    let typed = "swordfish-marmalade";
    type_password(&mut session, typed, now);

    assert!(!session.unlock(now));

    let message = session.prompt().expect("locked").error().expect("an error");
    assert!(!message.contains(typed), "leaked the password: {message}");
    assert!(
        !message.contains("swordfish"),
        "leaked a fragment: {message}"
    );
}

#[test]
fn typing_again_clears_the_previous_failure() {
    let (_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    type_password(&mut session, "nope", now);
    assert!(!session.unlock(now));
    assert!(session.prompt().expect("locked").error().is_some());

    session.type_char('a', now);
    assert!(session.prompt().expect("locked").error().is_none());
}

#[test]
fn missing_file_is_reported_as_such_not_as_a_wrong_password() {
    let dir = TempDir::new().expect("temp dir");
    let now = Instant::now();
    let mut session = KdbxSession::new(dir.path().join("absent.kdbx"));
    type_password(&mut session, VAULT_PASSWORD, now);

    assert!(!session.unlock(now));
    assert_eq!(
        session.prompt().expect("locked").error(),
        Some("File not found")
    );
}

#[test]
fn unlocking_an_unreadable_path_reports_a_read_error_not_a_password_error() {
    let dir = TempDir::new().expect("temp dir");
    // Reading a directory as a file fails even under root. Windows rejects
    // File::open; Unix rejects the subsequent read. A file in a non-directory
    // parent is unsuitable here: Windows reports that case as NotFound.
    let now = Instant::now();
    let mut session = KdbxSession::new(dir.path().to_path_buf());
    type_password(&mut session, VAULT_PASSWORD, now);

    assert!(!session.unlock(now));

    let error = session.prompt().expect("locked").error().expect("reason");
    assert!(error.starts_with("Cannot read file:"), "{error}");
}

#[test]
fn an_unreadable_key_file_reports_a_read_error_not_a_password_error() {
    let (_vault_dir, path) = vault_file(SAMPLE, VAULT_PASSWORD, None);
    let key_directory = TempDir::new().expect("temporary key directory");
    let now = Instant::now();
    let mut session = KdbxSession::new(path);
    session
        .prompt_mut()
        .expect("locked")
        .set_keyfile(Some(key_directory.path().to_path_buf()));
    type_password(&mut session, VAULT_PASSWORD, now);

    assert!(!session.unlock(now));
    let prompt = session.prompt().expect("still locked");
    assert!(prompt
        .error()
        .expect("reason")
        .starts_with("Cannot read file:"));
    assert!(prompt.is_empty(), "failed credentials are cleared");
}
