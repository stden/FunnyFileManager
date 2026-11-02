//! Purpose: The Windows PATH store against the real registry, read only: the
//! machine's PATH, which every Windows has and no test may change.

use super::*;

#[test]
fn the_machine_path_is_read_with_its_key_and_its_revision() {
    let snapshot = read(Scope::System).expect("the machine's PATH");
    assert_eq!(
        snapshot.location,
        r"HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Session Manager\Environment"
    );
    assert!(
        snapshot.value.to_ascii_lowercase().contains("system32"),
        "{:?}",
        snapshot.value
    );
    assert!(
        ["REG_EXPAND_SZ|", "REG_SZ|"]
            .iter()
            .any(|kind| snapshot.revision == format!("{kind}{}", snapshot.value)),
        "{:?}",
        snapshot.revision
    );
}

#[test]
fn the_current_user_is_told_where_its_path_lives() {
    assert_eq!(
        Place::of(Scope::User).location(),
        r"HKEY_CURRENT_USER\Environment"
    );
}

#[test]
fn place_failure_messages_and_stale_user_write_do_not_mutate_registry() {
    drop(read(Scope::User));
    let sys_denied =
        Place::of(Scope::System).failed(&io::Error::from(io::ErrorKind::PermissionDenied));
    assert!(sys_denied.to_string().contains("administrator"));
    let user_err = Place::of(Scope::User).failed(&io::Error::other("boom"));
    assert!(!user_err.to_string().contains("administrator"));

    let stale = Snapshot {
        revision: "STALE_NEVER_MATCHES_REGISTRY".to_owned(),
        value: String::new(),
        location: String::new(),
    };
    let err = write(Scope::User, &stale, r"C:\unused").expect_err("stale revision refused");
    assert!(err.to_string().contains(CHANGED_OUTSIDE), "{err}");
}
