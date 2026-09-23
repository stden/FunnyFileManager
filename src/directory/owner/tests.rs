//! Unit tests: the owner cache answers once per uid and once per gid.

use super::*;

#[cfg(unix)]
#[test]
fn the_running_user_resolves_and_is_cached() {
    let metadata = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("crate dir");
    let mut owners = Owners::default();
    let first = owners.of(&metadata);
    let second = owners.of(&metadata);
    assert_eq!(first, second);
    assert_eq!(owners.by_uid.len(), 1);
    assert_eq!(
        user_name(u32::MAX - 7),
        None,
        "a uid nobody has resolves to None"
    );
}

#[cfg(unix)]
#[test]
fn the_running_group_resolves_and_is_cached() {
    let metadata = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("crate dir");
    let mut owners = Owners::default();
    let first = owners.group_of(&metadata);
    assert!(first.is_some(), "the crate directory's group has a name");
    assert_eq!(owners.group_of(&metadata), first);
    assert_eq!(owners.by_gid.len(), 1);
    assert_eq!(
        group_name(u32::MAX - 7),
        None,
        "a gid nobody has resolves to None"
    );
}

#[cfg(unix)]
#[test]
fn user_and_group_joins_the_two_names() {
    let metadata = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("crate dir");
    let mut owners = Owners::default();
    let user = owners.of(&metadata).expect("the running user has a name");
    let group = owners.group_of(&metadata).expect("and so does the group");
    assert_eq!(
        owners.user_and_group(&metadata),
        Some(format!("{user}:{group}"))
    );
}

#[cfg(not(unix))]
#[test]
fn off_unix_there_is_no_owner() {
    let metadata = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("crate dir");
    assert_eq!(Owners::default().of(&metadata), None);
    assert_eq!(Owners::default().group_of(&metadata), None);
    assert_eq!(Owners::default().user_and_group(&metadata), None);
}
