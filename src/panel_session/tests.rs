//! Persistence is atomic, private, and skips unchanged panel pairs. A checkpoint
//! skips them; a flush of unchanged panels syncs the file on disk and never
//! replaces a session another FFM wrote.

use std::{fs, io};

use tempfile::TempDir;

use super::{PanelSession, Persistence};
use crate::file_write::SyncPolicy;

#[test]
fn saves_both_paths_and_updates_them_together() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("config/ffm/panel-session.json");
    let persistence = Persistence::File(file.clone());
    let mut session = PanelSession::load(persistence.clone());
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    session.save(&left, &right, SyncPolicy::Skip)?;
    session.save(&right, &left, SyncPolicy::Always)?;

    let loaded = PanelSession::load(persistence);
    let paths = loaded.paths().expect("saved directories");
    assert_eq!(paths.left, right);
    assert_eq!(paths.right, left);
    assert_eq!(fs::read_dir(file.parent().unwrap())?.count(), 1);
    Ok(())
}

#[test]
fn unchanged_panels_do_not_overwrite_a_newer_session() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("panel-session.json");
    let persistence = Persistence::File(file.clone());
    let mut older = PanelSession::load(persistence.clone());
    older.save(temp.path(), temp.path(), SyncPolicy::Skip)?;
    let mut newer = PanelSession::load(persistence);
    newer.save(&temp.path().join("next"), temp.path(), SyncPolicy::Skip)?;
    let latest = fs::read(&file)?;

    for policy in [SyncPolicy::Skip, SyncPolicy::Always] {
        older.save(temp.path(), temp.path(), policy)?;
        assert_eq!(fs::read(&file)?, latest, "{policy:?}");
    }
    Ok(())
}

#[test]
fn failed_write_can_retry_the_same_directories() -> io::Result<()> {
    let temp = TempDir::new()?;
    let blocked = temp.path().join("config");
    fs::write(&blocked, "not a directory")?;
    let file = blocked.join("panel-session.json");
    let mut session = PanelSession::load(Persistence::File(file.clone()));
    assert!(session
        .save(temp.path(), temp.path(), SyncPolicy::Skip)
        .is_err());
    assert!(session.paths().is_none());

    fs::remove_file(blocked)?;
    session.save(temp.path(), temp.path(), SyncPolicy::Skip)?;

    assert!(file.is_file());
    Ok(())
}

#[test]
fn session_only_never_writes_directories() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut session = PanelSession::default();
    session.save(temp.path(), temp.path(), SyncPolicy::Always)?;
    assert!(session.paths().is_none());
    assert_eq!(fs::read_dir(temp.path())?.count(), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn stored_paths_are_readable_only_by_the_owner() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new()?;
    let file = temp.path().join("panel-session.json");
    let mut session = PanelSession::load(Persistence::File(file.clone()));
    session.save(temp.path(), temp.path(), SyncPolicy::Skip)?;
    assert_eq!(fs::metadata(file)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}

/// The exit save of unchanged panels syncs the session file already there. When
/// the user deleted that file while FFM ran there is nothing to sync: the save
/// succeeds and writes no new file.
#[test]
fn flush_of_unchanged_panels_after_the_session_file_is_gone_writes_nothing() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("panel-session.json");
    let mut session = PanelSession::load(Persistence::File(file.clone()));
    session.save(temp.path(), temp.path(), SyncPolicy::Skip)?;
    fs::remove_file(&file)?;

    session.save(temp.path(), temp.path(), SyncPolicy::Always)?;

    assert!(!file.exists());
    assert_eq!(fs::read_dir(temp.path())?.count(), 0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn flush_refuses_a_fifo_without_waiting_for_a_reader() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("panel-session.json");
    let mut session = PanelSession::load(Persistence::File(file.clone()));
    session.save(temp.path(), temp.path(), SyncPolicy::Skip)?;
    fs::remove_file(&file)?;
    crate::test_support::make_fifo(&file)?;
    let directory = temp.path().to_owned();

    let result =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(2), move || {
            session.save(&directory, &directory, SyncPolicy::Always)
        })
        .expect("flush must not wait for a FIFO reader");
    assert_eq!(
        result.expect_err("FIFO is not a regular file").kind(),
        io::ErrorKind::InvalidInput
    );
    Ok(())
}
