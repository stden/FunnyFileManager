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
        user_name(Uid(u32::MAX - 7)),
        Lookup::NoEntry,
        "a uid nobody has has no entry"
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
        group_name(Gid(u32::MAX - 7)),
        Lookup::NoEntry,
        "a gid nobody has has no entry"
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

#[cfg(unix)]
#[test]
fn uid_zero_is_root() {
    assert_eq!(Owners::default().name_of_uid(0).as_deref(), Some("root"));
}

/// A uid nobody has is `None`, and it is remembered as such rather than
/// asked again on every row that carries it.
#[test]
fn an_unknown_uid_has_no_name_and_is_cached() {
    let mut owners = Owners::default();
    assert_eq!(owners.name_of_uid(u32::MAX - 7), None);
    assert_eq!(owners.name_of_uid(u32::MAX - 7), None);
    assert_eq!(owners.by_uid.len(), 1);
}

/// `of` and `name_of_uid` are one cache: a file's owner answers the process
/// that runs as the same uid without a second lookup.
#[cfg(unix)]
#[test]
fn of_and_name_of_uid_share_one_answer_per_uid() {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(env!("CARGO_MANIFEST_DIR")).expect("crate dir");
    let mut owners = Owners::default();
    let by_file = owners.of(&metadata);
    assert_eq!(owners.name_of_uid(metadata.uid()), by_file);
    assert_eq!(owners.by_uid.len(), 1);
}

/// A lookup that answers user7 for 7, no entry for 8 and fails for 9.
fn scripted(Uid(id): Uid) -> Lookup {
    match id {
        7 => Lookup::Named("user7".to_owned()),
        8 => Lookup::NoEntry,
        _ => Lookup::Failed,
    }
}

/// Asks `owners` for each of `ids` by uid, recording which ones reached the
/// lookup, and returns the names it answered.
fn ask(owners: &mut Owners, ids: &[u32], asked: &mut Vec<u32>) -> Vec<Option<String>> {
    ids.iter()
        .map(|&id| {
            cached(&mut owners.by_uid, Uid(id), |uid| {
                asked.push(uid.0);
                scripted(uid)
            })
        })
        .collect()
}

/// The lookup — an NSS call that may go to LDAP — runs once per id, however
/// often the id is asked for and whether it has a name, has none, or failed.
#[test]
fn each_id_is_looked_up_once_however_often_it_is_asked() {
    let mut owners = Owners::default();
    let mut asked = Vec::new();
    let answers = ask(&mut owners, &[7, 7, 8, 9, 7, 8, 9], &mut asked);
    let user7 = Some("user7".to_owned());
    assert_eq!(
        answers,
        [user7.clone(), user7.clone(), None, None, user7, None, None]
    );
    assert_eq!(asked, [7, 8, 9]);
}

/// A failed lookup is retried once its failure is forgotten; a name and a
/// missing entry are final. Without this, a long-lived reader would show one
/// transient NSS error as a bare uid until the session ends.
#[test]
fn only_a_failed_lookup_is_asked_again_after_forgetting_failures() {
    let mut owners = Owners::default();
    let mut asked = Vec::new();
    ask(&mut owners, &[7, 8, 9], &mut asked);
    owners.forget_failed_lookups();
    ask(&mut owners, &[7, 8, 9, 9], &mut asked);
    assert_eq!(asked, [7, 8, 9, 9]);
}

/// Forgetting failures covers the group cache too.
#[test]
fn forgetting_failures_retries_a_failed_group_lookup() {
    let mut owners = Owners::default();
    let mut asked = 0;
    let mut group = |owners: &mut Owners| {
        cached(&mut owners.by_gid, Gid(9), |_| {
            asked += 1;
            Lookup::Failed
        })
    };
    assert_eq!(group(&mut owners), None);
    assert_eq!(group(&mut owners), None);
    owners.forget_failed_lookups();
    assert_eq!(group(&mut owners), None);
    assert_eq!(asked, 2);
}

/// `getpwuid_r(3)`: 0 and the errnos it lists for "not found" are a missing
/// entry, kept for good; every other status is a failure to retry.
#[cfg(unix)]
#[test]
fn a_status_is_a_missing_entry_only_when_the_manual_says_so() {
    for status in [0, libc::ENOENT, libc::ESRCH, libc::EBADF, libc::EPERM] {
        assert_eq!(unnamed(status), Lookup::NoEntry, "status {status}");
    }
    for status in [
        libc::EIO,
        libc::EINTR,
        libc::EMFILE,
        libc::ENFILE,
        libc::ENOMEM,
        libc::ERANGE,
    ] {
        assert_eq!(unnamed(status), Lookup::Failed, "status {status}");
    }
}
