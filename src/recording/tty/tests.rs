//! Persistence is atomic, private, and skips unchanged panel pairs.

use std::{fs, io};

use tempfile::TempDir;

use super::{PanelSession, Persistence};

#[test]
fn saves_both_paths_and_updates_them_together() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("config/ffm/panel-session.json");
    let persistence = Persistence::File(file.clone());
    let mut session = PanelSession::load(persistence.clone());
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    session.save(&left, &right)?;
    session.save(&right, &left)?;

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
    older.save(temp.path(), temp.path())?;
    let mut newer = PanelSession::load(persistence);
    newer.save(&temp.path().join("next"), temp.path())?;
    let latest = fs::read(&file)?;

    older.save(temp.path(), temp.path())?;

    assert_eq!(fs::read(file)?, latest);
    Ok(())
}

#[test]
fn failed_write_can_retry_the_same_directories() -> io::Result<()> {
    let temp = TempDir::new()?;
    let blocked = temp.path().join("config");
    fs::write(&blocked, "not a directory")?;
    let file = blocked.join("panel-session.json");
    let mut session = PanelSession::load(Persistence::File(file.clone()));
    assert!(session.save(temp.path(), temp.path()).is_err());
    assert!(session.paths().is_none());

    fs::remove_file(blocked)?;
    session.save(temp.path(), temp.path())?;

    assert!(file.is_file());
    Ok(())
}

#[test]
fn session_only_never_writes_directories() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut session = PanelSession::default();
    session.save(temp.path(), temp.path())?;
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
    session.save(temp.path(), temp.path())?;
    assert_eq!(fs::metadata(file)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}
