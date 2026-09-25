//! Unit tests: "Preserve all timestamps" and "Access rights: Copy" reach
//! every entry a copied folder holds, not just the folder under the cursor.

use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, SystemTime};

use super::tests_transfer_policies::{app_between, mark, settings};
use super::*;

use crate::copy::AccessRights;
use crate::models::RequestedTimes;

use tempfile::TempDir;

/// The guest sweep's tree: `top` (0700) / `a` (0700) / `b` (0711) /
/// `secret.txt`, each a different age, the deepest first so that making
/// one does not touch the time of the folder above it.
pub(super) const TREE: [(&str, u32, u64); 4] = [
    ("top/a/b/secret.txt", 0o600, 4),
    ("top/a/b", 0o711, 3),
    ("top/a", 0o700, 2),
    ("top", 0o700, 1),
];

fn days_ago(days: u64) -> SystemTime {
    SystemTime::now() - Duration::from_hours(24 * days)
}

pub(super) fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).and_then(|meta| meta.modified()).unwrap()
}

pub(super) fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

/// The tree in `from`, marked, and an empty `to`.
pub(super) fn tree() -> (App, TempDir, TempDir) {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::create_dir_all(from.path().join("top/a/b")).expect("test setup");
    fs::write(from.path().join("top/a/b/secret.txt"), b"secret").expect("test setup");
    for (name, bits, age) in TREE {
        let path = from.path().join(name);
        let when = Some(days_ago(age));
        let times = RequestedTimes {
            modified: when,
            accessed: when,
        };
        crate::ops::timestamps::set_times(&path, times).expect("test setup");
        fs::set_permissions(&path, fs::Permissions::from_mode(bits)).expect("test setup");
    }
    let mut app = app_between(from.path(), to.path());
    app.get_active_panel().load_entries().expect("reload");
    mark(&mut app, "top");
    (app, from, to)
}

#[test]
fn preserving_timestamps_reaches_every_file_and_folder_inside() {
    let (mut app, from, to) = tree();
    let mut dialog = settings(to.path());
    dialog.timestamps = true;
    let accessed = |path: &Path| fs::metadata(path).and_then(|meta| meta.accessed()).unwrap();
    let secret = "top/a/b/secret.txt";
    // Read before the copy: reading the source is what updates its access
    // time on a `relatime` mount (CI's), while the copy carries the old one.
    let accessed_before = accessed(&from.path().join(secret));

    copy_with(&mut app, &dialog);

    for (name, _, _) in TREE {
        let (source, copy) = (from.path().join(name), to.path().join(name));
        assert_eq!(modified(&copy), modified(&source), "{name}: {}", app.status);
    }
    assert_eq!(accessed(&to.path().join(secret)), accessed_before);
}

#[test]
fn copying_access_rights_reaches_every_folder_inside() {
    let (mut app, from, to) = tree();
    let mut dialog = settings(to.path());
    dialog.rights = AccessRights::Copy;

    copy_with(&mut app, &dialog);

    for (name, _, _) in TREE {
        let (source, copy) = (from.path().join(name), to.path().join(name));
        assert_eq!(mode(&copy), mode(&source), "{name}: {}", app.status);
    }
}

/// Give the link at `link` itself the time `when`, as `touch -h` does.
fn stamp_link(link: &Path, when: SystemTime) {
    use rustix::fs::{utimensat, AtFlags, Timestamps, CWD};

    let time = crate::utils::timespec(when).expect("test setup");
    let (last_access, last_modification) = (time, time);
    let stamps = Timestamps {
        last_access,
        last_modification,
    };
    utimensat(CWD, link, &stamps, AtFlags::SYMLINK_NOFOLLOW).expect("test setup");
}

/// A link's own modification time, not its target's.
fn own_modified(path: &Path) -> SystemTime {
    fs::symlink_metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap()
}

/// A copied link keeps its own times, and whatever it points at keeps its
/// own: setting them through the link would restamp an unrelated file.
#[test]
fn preserving_timestamps_stamps_a_link_itself_never_what_it_points_at() {
    let from = TempDir::new().expect("test setup");
    let to = TempDir::new().expect("test setup");
    fs::create_dir(from.path().join("top")).expect("test setup");
    let link = from.path().join("top/link");
    std::os::unix::fs::symlink("../outside.txt", &link).expect("test setup");
    stamp_link(&link, days_ago(9));
    // What the copied link will point at, sitting in the destination.
    let outside = to.path().join("outside.txt");
    fs::write(&outside, b"theirs").expect("test setup");
    let untouched = modified(&outside);
    let mut app = app_between(from.path(), to.path());
    mark(&mut app, "top");
    let mut dialog = settings(to.path());
    dialog.timestamps = true;

    copy_with(&mut app, &dialog);

    let copied = own_modified(&to.path().join("top/link"));
    assert_eq!(copied, own_modified(&link), "{}", app.status);
    assert_eq!(modified(&outside), untouched);
}
