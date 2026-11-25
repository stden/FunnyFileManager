//! Purpose: Unknown variable expansion and missing-parent errors in Unix PATH persistence.

use super::*;
use tempfile::TempDir;

#[test]
fn unknown_env_variable_and_parentless_path_are_rejected() {
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join("profile");
    fs::write(&path, "export PATH=/bin\n").expect("fixture");
    let snapshot = read_at(&path, Format::Profile, "/bin").expect("snapshot");

    let err = write_expanded_at(
        &path,
        Format::Profile,
        &snapshot,
        "$FFM_DEFINITELY_UNKNOWN_VAR_999/bin",
    )
    .expect_err("unknown variable must fail");
    assert!(err.to_string().contains("Unknown PATH variable"), "{err}");

    let err = replace_file(Path::new("/"), "", 0o644).expect_err("root has no parent");
    assert!(err.to_string().contains("no parent"), "{err}");
}
