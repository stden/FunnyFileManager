//! Exhaustion tests for the transfer retry loops: when every destination is
//! claimed by a (simulated) racing process, the loops must give up with
//! `AlreadyExists` after `MAX_TRANSFER_ATTEMPTS` instead of spinning forever.

use std::fs;
use std::sync::atomic::Ordering;

use tempfile::TempDir;

use super::{
    copy_into, move_into, FORCE_RENAME_FAILURE, FORCE_TRANSFER_COLLISION, RENAME_TEST_MUTEX,
};

/// RAII reset so a panicking assertion cannot leak the collision flag into the
/// rest of this thread's tests.
struct CollisionGuard;

impl CollisionGuard {
    fn engage() -> Self {
        FORCE_TRANSFER_COLLISION.set(true);
        Self
    }
}

impl Drop for CollisionGuard {
    fn drop(&mut self) {
        FORCE_TRANSFER_COLLISION.set(false);
    }
}

#[test]
fn copy_gives_up_after_every_destination_collides() {
    let source_dir = TempDir::new().expect("source dir");
    let target_dir = TempDir::new().expect("target dir");
    let source = source_dir.path().join("data.txt");
    fs::write(&source, "payload").expect("write source");

    let _collide = CollisionGuard::engage();
    let error = copy_into(&source, target_dir.path()).expect_err("must exhaust");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(
        error
            .to_string()
            .contains("could not find exclusive destination"),
        "{error}"
    );
}

#[test]
fn move_gives_up_after_every_destination_collides() {
    // `FORCE_RENAME_FAILURE` is process-global, so serialize with the other
    // tests that flip it; the collision flag itself is thread-local.
    let _lock = RENAME_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let source_dir = TempDir::new().expect("source dir");
    let target_dir = TempDir::new().expect("target dir");
    let source = source_dir.path().join("data.txt");
    fs::write(&source, "payload").expect("write source");

    // Rename must fail too, or the move never reaches its copy fallback.
    FORCE_RENAME_FAILURE.store(true, Ordering::SeqCst);
    let _collide = CollisionGuard::engage();
    let error = move_into(&source, target_dir.path()).expect_err("must exhaust");
    FORCE_RENAME_FAILURE.store(false, Ordering::SeqCst);

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(
        source.exists(),
        "an exhausted move must not lose the source"
    );
}
