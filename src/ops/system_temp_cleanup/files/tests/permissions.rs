//! Purpose: Real OS denials preserve temporary files and report partial scans honestly.

use super::*;
use crate::test_support::{filesystem_faults, ModeGuard};

#[test]
fn an_unreadable_root_reports_scan_failure() {
    filesystem_faults::run(|| {
        let root = tempfile::TempDir::new().unwrap();
        let _mode = ModeGuard::set(root.path(), 0o300).unwrap();
        assert!(matches!(
            scan(&source(root.path()), SystemTime::now()),
            Err(TempError::Scan)
        ));
    });
}

#[test]
fn an_unreadable_subfolder_is_skipped_without_abandoning_other_files() {
    filesystem_faults::run(|| {
        let root = tempfile::TempDir::new().unwrap();
        let hidden = root.path().join("hidden");
        fs::create_dir(&hidden).unwrap();
        let now = SystemTime::now();
        old_file(&root.path().join("old"), b"old", now);
        let _mode = ModeGuard::set(&hidden, 0o300).unwrap();
        let plan = scan(&source(root.path()), now).unwrap();
        assert_eq!(plan.skipped, 1);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.bytes.bytes(), 3);
    });
}

#[test]
fn an_unlink_denial_does_not_claim_to_have_freed_the_file() {
    filesystem_faults::run(|| {
        let root = tempfile::TempDir::new().unwrap();
        let file = root.path().join("old");
        let now = SystemTime::now();
        old_file(&file, b"keep", now);
        let plan = scan(&source(root.path()), now).unwrap();
        let _mode = ModeGuard::set(root.path(), 0o555).unwrap();
        let outcome = remove(&plan);
        assert_eq!(outcome.failures, 1);
        assert_eq!(outcome.removed, 0);
        assert_eq!(outcome.freed.bytes(), 0);
        assert_eq!(fs::read(file).unwrap(), b"keep");
    });
}

#[test]
fn the_system_source_resolves_only_the_private_child_temporary_directory() {
    filesystem_faults::run(|| {
        let now = SystemTime::now();
        let plan = scan(&TempSource::System, now).unwrap();
        assert_eq!(
            plan.root.path(),
            fs::canonicalize(std::env::temp_dir()).unwrap()
        );
        assert_eq!(plan.files.len(), 0);
    });
}
