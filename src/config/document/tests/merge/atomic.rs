//! Unit tests: how a save writes — under the lock another FFM may hold, and
//! through a private temp file renamed over the old one.

use super::*;

/// Two FFMs saving at once: the second waits for no one and says why.
#[test]
fn a_save_is_refused_while_another_ffm_holds_the_lock() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let lock = File::create(temp.path().join("ffm").join("settings.yaml.lock")).expect("lock file");
    lock.lock().expect("held by the other FFM");
    let loaded = Counter { counter: 1 };
    let current = Counter { counter: 2 };
    let error = update(&path, &loaded, &current).expect_err("locked");
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(
        error.to_string(),
        "Not saved: another FFM is saving settings.yaml"
    );
    assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
    drop(lock);
    update(&path, &loaded, &current).expect("free again");
    assert_eq!(document(&path).get("counter"), Some(&Value::from(2)));
}

/// A program FFM starts on another thread shares the lock from its fork
/// until its exec, so a lock this FFM has just let go of can still read as
/// held for a moment: the next save refused, "another FFM is saving". A lock
/// freed a moment later is waited for.
#[test]
fn a_lock_freed_a_moment_later_is_waited_for() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    let lock = File::create(temp.path().join("ffm").join("settings.yaml.lock")).expect("lock file");
    lock.lock().expect("held for a moment");
    let freed = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        drop(lock);
    });
    update(&path, &Counter { counter: 1 }, &Counter { counter: 2 }).expect("saved");
    freed.join().expect("the lock is freed");
    assert_eq!(document(&path).get("counter"), Some(&Value::from(2)));
}

#[test]
fn a_lock_that_cannot_be_opened_refuses_the_save() {
    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    fs::create_dir_all(temp.path().join("ffm").join("settings.yaml.lock"))
        .expect("a directory where the lock goes");
    let error = save(&path, 2).expect_err("no lock");
    assert!(
        error
            .to_string()
            .starts_with("Not saved: settings.yaml.lock: "),
        "{error}"
    );
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn a_save_renames_a_private_temp_file_over_the_old_one() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp = TempDir::new().expect("tempdir");
    let path = settings_path(&temp);
    write_file(&path, "counter: 1\n");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");
    let before = fs::metadata(&path).expect("stat").ino();
    update(&path, &Counter { counter: 1 }, &Counter { counter: 2 }).expect("save");
    let after = fs::metadata(&path).expect("stat");
    assert_ne!(after.ino(), before, "a new file replaced the old one");
    assert_eq!(after.permissions().mode() & 0o777, 0o600);
    let mut names: Vec<_> = fs::read_dir(temp.path().join("ffm"))
        .expect("config dir")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [FILE_NAME, "settings.yaml.lock"],
        "no temp file is left behind"
    );
}
