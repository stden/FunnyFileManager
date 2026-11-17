use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn write_errors_are_reported_without_counting_an_uncommitted_replacement() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("locked");
        fs::create_dir(&directory).unwrap();
        let file = directory.join("text");
        fs::write(&file, "xxx").unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o555)).unwrap();
        let mut run = ReplaceRun::default();
        replace_recursive(root.path(), &options(false), &mut run);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(run.errors.len(), 1);
        assert_eq!(run.total, 0);
        assert_eq!(run.changed, Vec::<serde_json::Value>::new());
        assert_eq!(fs::read_to_string(file).unwrap(), "xxx");
    });
}

#[test]
fn unreadable_directories_are_skipped_and_cannot_create_false_replacements() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("closed");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("text"), "xxx").unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o000)).unwrap();
        let mut run = ReplaceRun::default();
        replace_recursive(root.path(), &options(false), &mut run);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(run.changed, Vec::<serde_json::Value>::new());
        assert_eq!(fs::read_to_string(directory.join("text")).unwrap(), "xxx");
    });
}
