//! Purpose: A real permission refusal inside a copied tree names the entry it stopped at.

use super::*;
use tempfile::TempDir;

#[test]
fn a_locked_copy_destination_is_named_in_the_tree_error() {
    crate::test_support::filesystem_faults::run(|| {
        let source = TempDir::new().expect("source");
        let dest = TempDir::new().expect("destination");
        fs::write(source.path().join("file"), b"whole").expect("source file");
        let _locked = crate::test_support::ModeGuard::set(dest.path(), 0o555).expect("lock");

        let error = copy_dir_all(source.path(), dest.path()).expect_err("locked destination");

        let failed = FailedAt::of(&error).expect("the refusal names its entry");
        assert_eq!(failed.path(), dest.path().join("file"));
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            fs::read(source.path().join("file")).expect("source"),
            b"whole"
        );
    });
}
