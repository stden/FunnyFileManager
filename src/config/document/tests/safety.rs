//! Unit tests: the files no save may replace — a link that leads nowhere,
//! a file with other hard links, another user's file, a read-only one — and
//! the link every save writes through. Each runs through both ways a save
//! reaches the file, so every writer is held to them.

use super::*;

/// The two ways a save reaches the file.
#[derive(Debug, Clone, Copy)]
enum Writer {
    /// A writer's three-way save of what it changed (`counter` 1 → 2).
    Update,
    /// An edit of the file's own mapping (`counter` to 2).
    Edit,
}

impl Writer {
    const ALL: [Self; 2] = [Self::Update, Self::Edit];

    /// Save `counter: 2` into the file at `path`, which holds `counter: 1`.
    fn save(self, path: &Path) -> io::Result<()> {
        match self {
            Self::Update => update(path, &Counter { counter: 1 }, &Counter { counter: 2 }),
            Self::Edit => {
                let _: Counter = edit(path, |document, _: &Counter| {
                    document.insert("counter".into(), Value::from(2));
                    Ok(())
                })?;
                Ok(())
            }
        }
    }
}

/// A settings file kept in a dotfiles repository: the link stays a link, the
/// file it names gets the change, and the lock sits beside the link.
#[cfg(unix)]
#[test]
fn a_symlink_is_written_through_and_stays_a_link() {
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        let real = temp.path().join("dotfiles").join(FILE_NAME);
        write_file(&real, "counter: 1\n");
        fs::create_dir_all(path.parent().expect("a parent")).expect("config dir");
        std::os::unix::fs::symlink(&real, &path).expect("link");
        writer.save(&path).expect("saved");
        let link = fs::symlink_metadata(&path).expect("stat");
        assert!(link.is_symlink(), "{writer:?}");
        assert_eq!(
            stored(&real).parse::<Counter>().expect("YAML"),
            Counter { counter: 2 },
            "{writer:?}"
        );
        assert!(path.with_extension("yaml.lock").exists(), "{writer:?}");
        assert!(!real.with_extension("yaml.lock").exists(), "{writer:?}");
    }
}

/// Writing to a link that leads nowhere would turn it into a plain file.
#[cfg(unix)]
#[test]
fn a_dangling_link_is_refused_and_stays_a_link() {
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        let missing = temp.path().join("gone.yaml");
        fs::create_dir_all(path.parent().expect("a parent")).expect("config dir");
        std::os::unix::fs::symlink(&missing, &path).expect("link");
        let error = writer.save(&path).expect_err("refused");
        assert_eq!(error.kind(), io::ErrorKind::NotFound, "{writer:?}");
        assert_eq!(
            error.to_string(),
            format!(
                "Not saved: settings.yaml: the link points at {}, which does not exist",
                missing.display()
            )
        );
        let link = fs::symlink_metadata(&path).expect("stat");
        assert!(link.is_symlink(), "{writer:?}");
        assert!(!missing.exists(), "{writer:?}");
    }
}

/// A rename detaches only the name it replaces: the other name would keep
/// the old text.
#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_refused_and_both_names_keep_the_text() {
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        write_file(&path, "counter: 1\n");
        let other = temp.path().join("other.yaml");
        fs::hard_link(&path, &other).expect("hard link");
        let error = writer.save(&path).expect_err("refused");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{writer:?}");
        assert_eq!(
            error.to_string(),
            "Not saved: settings.yaml has other hard links; FFM will not replace it"
        );
        for name in [&path, &other] {
            assert_eq!(fs::read_to_string(name).expect("kept"), "counter: 1\n");
        }
    }
}

/// Under `sudo` with the user's home, a rename would hand the user's file to
/// root, readable by root only. Only root can give a file away and still
/// write it, so only root can set this up.
#[cfg(unix)]
#[test]
fn another_users_file_is_refused_and_keeps_its_text_and_owner() {
    use std::os::unix::fs::MetadataExt as _;

    const ANOTHER_UID: u32 = 4_242_424;
    if !crate::platform::is_root() {
        return;
    }
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        write_file(&path, "counter: 1\n");
        std::os::unix::fs::chown(&path, Some(ANOTHER_UID), None).expect("chown");
        let error = writer.save(&path).expect_err("refused");
        assert_eq!(
            error.to_string(),
            "Not saved: settings.yaml belongs to another user; FFM will not replace it"
        );
        assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
        assert_eq!(fs::metadata(&path).expect("stat").uid(), ANOTHER_UID);
    }
}

/// A rename ignores the file's own permissions; the save must not. Root may
/// write anything, so the refusal can only be seen as another user.
#[cfg(unix)]
#[test]
fn a_read_only_file_is_refused() {
    use std::os::unix::fs::PermissionsExt as _;

    if crate::platform::is_root() {
        return;
    }
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        write_file(&path, "counter: 1\n");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).expect("chmod");
        let error = writer.save(&path).expect_err("refused");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{writer:?}");
        assert!(error.to_string().starts_with(NOT_SAVED), "{error}");
        assert_eq!(fs::read_to_string(&path).expect("kept"), "counter: 1\n");
    }
}

/// The refusals leave an ordinary file alone: a plain file this user owns,
/// with one name, is saved by both writers.
#[test]
fn an_ordinary_file_is_saved_by_both_writers() {
    for writer in Writer::ALL {
        let temp = TempDir::new().expect("tempdir");
        let path = settings_path(&temp);
        write_file(&path, "counter: 1\n");
        writer.save(&path).expect("saved");
        assert_eq!(
            stored(&path).parse::<Counter>().expect("YAML"),
            Counter { counter: 2 },
            "{writer:?}"
        );
    }
}
