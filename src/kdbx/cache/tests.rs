use super::*;
use crate::kdbx::{tests_support::unlocked_sample, IDLE_LOCK};

#[test]
fn reopen_remembers_the_key_but_expiry_and_cloning_forget_it() {
    let (_dir, session, now) = unlocked_sample();
    let path = session.path.clone();
    let mut cache = SessionCache::default();
    cache.remember(session);
    assert!(cache
        .clone()
        .take(path.clone(), Access::View, now)
        .state
        .is_locked());
    let session = cache.take(path.clone(), Access::View, now);
    assert!(!session.state.is_locked());
    cache.remember(session);
    assert!(cache
        .take(path, Access::Edit, now + IDLE_LOCK)
        .state
        .is_locked());
}

#[test]
fn a_replaced_vault_cannot_be_opened_with_the_cached_credentials() {
    let (_dir, session, now) = unlocked_sample();
    let path = session.path.clone();
    let mut cache = SessionCache::default();
    cache.remember(session);
    std::fs::write(&path, b"changed").expect("replace");
    assert!(cache.take(path, Access::View, now).state.is_locked());
}

#[test]
fn the_cache_forgets_the_oldest_key_when_its_capacity_is_reached() {
    let mut directories = Vec::new();
    let mut paths = Vec::new();
    let mut cache = SessionCache::default();
    for _ in 0..17 {
        let (directory, session, _) = unlocked_sample();
        paths.push(session.path.clone());
        directories.push(directory);
        cache.remember(session);
    }
    let now = Instant::now();
    assert!(cache
        .take(paths[0].clone(), Access::View, now)
        .state
        .is_locked());
    assert!(!cache
        .take(paths[16].clone(), Access::View, now)
        .state
        .is_locked());
    assert!(directories
        .iter()
        .all(|directory| directory.path().is_dir()));
}

#[test]
fn cancelled_unlocks_do_not_displace_a_remembered_key() {
    let (_directory, session, now) = unlocked_sample();
    let path = session.path.clone();
    let mut cache = SessionCache::default();
    cache.remember(session);
    cache.remember(KdbxSession::new(path.clone()));
    assert!(!cache.take(path, Access::View, now).state.is_locked());
}
