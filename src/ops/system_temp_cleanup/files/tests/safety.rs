//! Purpose: Safety regressions keep replaced, locked and out-of-root files intact.
use super::*;

#[test]
fn a_same_size_replacement_with_the_same_timestamp_is_not_deleted() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temp");
    fs::create_dir(&root).unwrap();
    let now = SystemTime::now();
    old_file(&root.join("file"), b"old", now);
    let plan = scan(&source(&root), now).unwrap();
    fs::rename(root.join("file"), temp.path().join("original")).unwrap();
    fs::write(root.join("file"), b"new").unwrap();
    fs::File::options()
        .write(true)
        .open(root.join("file"))
        .unwrap()
        .set_modified(plan.files[0].modified)
        .unwrap();
    let outcome = remove(&plan);
    assert_eq!(outcome.removed, 0);
    assert_eq!(fs::read(root.join("file")).unwrap(), b"new");
    assert_eq!(fs::read(temp.path().join("original")).unwrap(), b"old");
}
#[test]
fn a_locked_old_file_is_left_in_place_without_claiming_freed_bytes() {
    let temp = tempfile::TempDir::new().unwrap();
    let now = SystemTime::now();
    let path = temp.path().join("active");
    old_file(&path, b"active", now);
    let plan = scan(&source(temp.path()), now).unwrap();
    let lock = fs::File::options()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    lock.try_lock().unwrap();
    let outcome = remove(&plan);
    assert_eq!(outcome.removed, 0);
    assert_eq!(outcome.freed.bytes(), 0);
    drop(lock);
    assert_eq!(fs::read(path).unwrap(), b"active");
}
#[test]
fn unavailable_and_home_roots_are_refused_before_a_plan_exists() {
    let temp = tempfile::TempDir::new().unwrap();
    let now = SystemTime::now();
    assert!(matches!(
        scan(&TempSource::Unavailable, now),
        Err(TempError::Unavailable)
    ));
    let home = TempSource::Directory {
        path: temp.path().to_owned(),
        home: Some(temp.path().to_owned()),
    };
    assert!(matches!(scan(&home, now), Err(TempError::UnsafeRoot)));
}
#[cfg(unix)]
#[test]
fn links_to_outside_files_and_sockets_are_not_cleanup_candidates() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temp");
    let outside = temp.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    let now = SystemTime::now();
    old_file(&outside.join("keep"), b"keep", now);
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.join("active.sock")).unwrap();
    let plan = scan(&source(&root), now).unwrap();
    assert_eq!(plan.files.len(), 0);
    assert_eq!(remove(&plan).freed.bytes(), 0);
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
    assert!(root.join("active.sock").exists());
}
#[cfg(unix)]
#[test]
fn replacing_a_scanned_parent_with_a_link_cannot_delete_its_outside_target() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temp");
    fs::create_dir_all(root.join("nested")).unwrap();
    let now = SystemTime::now();
    old_file(&root.join("nested/keep"), b"keep", now);
    let plan = scan(&source(&root), now).unwrap();
    assert_eq!(plan.files.len(), 1);
    let outside = temp.path().join("outside");
    fs::rename(root.join("nested"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("nested")).unwrap();
    assert_eq!(remove(&plan).removed, 0);
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
}
