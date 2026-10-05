//! Unit tests: a file only its owner may read is replaced atomically and
//! owner-only, through a symbolic link, and never over a dangling link or a
//! hard-linked file.

use std::fs;
use std::io;

use tempfile::TempDir;

use super::replace_owner_only;

#[test]
fn a_new_or_ordinary_file_is_written_owner_only() -> io::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("bookmarks");
    replace_owner_only(&path, "bookmarks", "1 /srv\n")?;
    replace_owner_only(&path, "bookmarks", "2 /srv\n")?;
    assert_eq!(fs::read_to_string(&path)?, "2 /srv\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_link_is_written_through_and_a_dangling_one_is_refused() -> io::Result<()> {
    let dir = TempDir::new()?;
    let real = dir.path().join("dotfiles-history");
    fs::write(&real, "old\n")?;
    let link = dir.path().join("history");
    std::os::unix::fs::symlink(&real, &link)?;
    replace_owner_only(&link, "history", "new\n")?;
    assert!(fs::symlink_metadata(&link)?.is_symlink());
    assert_eq!(fs::read_to_string(&real)?, "new\n");
    let dangling = dir.path().join("dangling");
    std::os::unix::fs::symlink(dir.path().join("gone"), &dangling)?;
    let error = replace_owner_only(&dangling, "history", "x").expect_err("dangling");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(
        error
            .to_string()
            .starts_with("history: the link points at "),
        "{error}"
    );
    assert!(fs::symlink_metadata(&dangling)?.is_symlink());
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_hard_linked_file_is_refused_with_its_name() -> io::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("panel-session.json");
    fs::write(&path, "{}")?;
    fs::hard_link(&path, dir.path().join("other"))?;
    let error = replace_owner_only(&path, "panel-session.json", "[]").expect_err("linked");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(
        error.to_string(),
        "panel-session.json has other hard links; FFM will not replace it"
    );
    assert_eq!(fs::read_to_string(&path)?, "{}");
    Ok(())
}

/// The XDG spec asks for its directories to be created 0700; they were
/// created 0755, so every other user could list the names of the user's
/// settings, histories and LSP cache. A folder that is already there keeps
/// its mode: how it is shared is its owner's choice.
#[cfg(unix)]
#[test]
fn missing_folders_are_made_owner_only_and_existing_ones_keep_their_mode() -> io::Result<()> {
    use super::create_dirs_owner_only;
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new()?;
    let shared = dir.path().join("shared");
    fs::create_dir(&shared)?;
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755))?;
    let ffm = shared.join(".config").join("ffm");
    create_dirs_owner_only(&ffm)?;
    create_dirs_owner_only(&ffm)?;
    let mode = |path: &std::path::Path| {
        fs::metadata(path).map(|metadata| metadata.permissions().mode() & 0o777)
    };
    assert_eq!(mode(&shared)?, 0o755, "left as it was");
    assert_eq!(mode(&shared.join(".config"))?, 0o700);
    assert_eq!(mode(&ffm)?, 0o700);
    Ok(())
}
