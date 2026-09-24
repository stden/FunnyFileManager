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
