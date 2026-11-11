use super::*;
use crate::tree_walk::{
    append_tree, directory_size_with_budget, entry_size_for_type, TREE_WALK_BUDGET,
};

#[cfg(unix)]
#[path = "../../../../tests/support/permission_process.rs"]
mod faults;

#[cfg(unix)]
#[test]
fn unreadable_subdirectories_contribute_no_bytes() {
    use std::os::unix::fs::PermissionsExt;
    faults::run(|| {
        let root = TempDir::new().unwrap();
        let denied = root.path().join("denied");
        fs::create_dir(&denied).unwrap();
        fs::write(denied.join("private"), b"unreadable").unwrap();
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).unwrap();
        let size = directory_size(root.path());
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(size, 0);
    });
}

#[cfg(windows)]
#[test]
fn exclusively_locked_subdirectories_contribute_no_bytes() {
    use std::os::windows::fs::OpenOptionsExt;

    let root = TempDir::new().unwrap();
    let denied = root.path().join("denied");
    fs::create_dir(&denied).unwrap();
    fs::write(denied.join("private"), b"unreadable").unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(0x0200_0000)
        .open(&denied)
        .unwrap();
    let size = directory_size(root.path());
    drop(lock);
    assert_eq!(size, 0);
}

#[test]
fn size_walk_honors_zero_and_exhausted_budgets() {
    let root = TempDir::new().unwrap();
    for name in ["first", "second"] {
        fs::create_dir(root.path().join(name)).unwrap();
        fs::write(root.path().join(name).join("bytes"), "payload").unwrap();
    }
    assert_eq!(directory_size_with_budget(root.path(), 0), 0);
    assert_eq!(directory_size_with_budget(root.path(), 1), 0);
    assert_eq!(directory_size(root.path()), 14);
}

#[test]
fn metadata_fallback_handles_missing_type_and_a_disappeared_entry() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("file"), "bytes").unwrap();
    fs::create_dir(root.path().join("dir")).unwrap();
    for entry in fs::read_dir(root.path()).unwrap().flatten() {
        let mut pending = Vec::new();
        let size = entry_size_for_type(
            &entry,
            &Err(std::io::ErrorKind::Unsupported.into()),
            &mut pending,
        );
        if entry.file_name() == "dir" {
            assert_eq!(size, 0);
            assert_eq!(pending, [entry.path()]);
        } else {
            assert_eq!(size, 5);
            assert_eq!(pending, Vec::<std::path::PathBuf>::new());
            fs::remove_file(entry.path()).unwrap();
            assert_eq!(
                entry_size_for_type(
                    &entry,
                    &Err(std::io::ErrorKind::NotFound.into()),
                    &mut pending
                ),
                0
            );
            assert_eq!(pending, Vec::<std::path::PathBuf>::new());
        }
    }
}

#[test]
fn rendering_stops_before_the_first_entry_over_its_budget() {
    let root = TempDir::new().unwrap();
    for name in ["first", "second"] {
        fs::write(root.path().join(name), "bytes").unwrap();
    }
    let mut visited = TREE_WALK_BUDGET - 1;
    let mut output = String::new();
    append_tree(
        root.path(),
        &mut String::new(),
        0,
        1,
        &mut visited,
        &mut output,
    );
    assert_eq!(visited, TREE_WALK_BUDGET);
    assert_eq!(output.lines().filter(|line| !line.is_empty()).count(), 1);
}
