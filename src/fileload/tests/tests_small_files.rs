//! Unit tests: a configuration or data file is read with the guards of every
//! file FFM keeps, and [`open_regular`] opens a lock or a log without ever
//! waiting on a FIFO in its place.

use super::super::{open_regular, read_small_file, MAX_SMALL_FILE_BYTES};
use super::*;

use std::fs::OpenOptions;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
const GUARD_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn a_small_file_is_read_and_every_refusal_names_it() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("history");
    fs::write(&path, "cd /srv\n")?;
    assert_eq!(read_small_file(&path, "history")?, "cd /srv\n");
    let refusal = |path: &std::path::Path| {
        read_small_file(path, "history").map_err(|error| (error.kind(), error.to_string()))
    };
    assert_eq!(
        refusal(temp.path()),
        Err((
            io::ErrorKind::Other,
            "history is not a regular file".to_owned()
        ))
    );
    File::create(&path)?.set_len(MAX_SMALL_FILE_BYTES + 1)?;
    assert_eq!(
        refusal(&path),
        Err((io::ErrorKind::Other, "history exceeds 1 MiB".to_owned()))
    );
    fs::write(&path, b"\xff")?;
    assert!(
        matches!(refusal(&path), Err((io::ErrorKind::InvalidData, message))
        if message.starts_with("history: not UTF-8 text"))
    );
    let missing = refusal(&temp.path().join("gone"));
    assert!(matches!(missing, Err((io::ErrorKind::NotFound, message))
        if message.starts_with("history: ")));
    Ok(())
}

/// A lock or a log is created when missing, and a regular one is opened.
#[test]
fn open_regular_creates_a_missing_file_and_opens_a_regular_one() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("settings.yaml.lock");
    let create = OpenOptions::new().write(true).create(true).clone();
    open_regular(&path, &create)?;
    assert!(path.is_file());
    open_regular(&path, &create)?;
    let error = open_regular(temp.path(), OpenOptions::new().read(true)).expect_err("a folder");
    assert_eq!(error.to_string(), "not a regular file");
    let missing = open_regular(&temp.path().join("gone"), OpenOptions::new().read(true));
    assert_eq!(
        missing.map_err(|error| error.kind()).err(),
        Some(io::ErrorKind::NotFound)
    );
    Ok(())
}

/// A path the type check cannot even look at — through a file — is refused
/// with the reason, and nothing is created.
#[cfg(unix)]
#[test]
fn open_regular_refuses_a_path_it_cannot_look_at() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("file");
    fs::write(&file, "")?;
    let create = OpenOptions::new().write(true).create(true).clone();
    let error = open_regular(&file.join("lock"), &create).expect_err("through a file");
    assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
    Ok(())
}

/// A FIFO where a lock or a data file goes: opening it for writing waits for
/// a reader and for reading waits for a writer. Neither open may wait.
#[cfg(unix)]
#[test]
fn a_fifo_is_refused_for_reading_and_for_writing_without_waiting() -> io::Result<()> {
    let temp = TempDir::new()?;
    let fifo = temp.path().join("pipe");
    crate::test_support::make_fifo(&fifo)?;
    let outcome = crate::test_support::run_with_timeout(GUARD_TIMEOUT, move || {
        let write = OpenOptions::new().write(true).create(true).clone();
        [
            open_regular(&fifo, &write)
                .map(drop)
                .map_err(|error| error.to_string()),
            open_regular(&fifo, OpenOptions::new().read(true))
                .map(drop)
                .map_err(|error| error.to_string()),
            read_small_file(&fifo, "pipe")
                .map(drop)
                .map_err(|error| error.to_string()),
        ]
    })
    .expect("no open waits on the FIFO");
    assert_eq!(
        outcome,
        [
            Err("not a regular file".to_owned()),
            Err("not a regular file".to_owned()),
            Err("pipe is not a regular file".to_owned()),
        ]
    );
    Ok(())
}

/// The data files FFM keeps under `$XDG_DATA_HOME/ffm`, and the panel session.
#[cfg(unix)]
const DATA_FILES: [&str; 5] = [
    "history",
    "bookmarks",
    "folder_history",
    "view_history",
    "panel-session.json",
];

/// The data files FFM keeps — the command, folder and view histories, the
/// bookmarks and the panel session: a FIFO in the place of any of them loads
/// as nothing, and loading does not wait on it.
#[cfg(unix)]
#[test]
fn a_fifo_in_place_of_a_data_file_loads_as_nothing_without_waiting() -> io::Result<()> {
    use crate::models::{Bookmarks, FolderHistory, ViewHistory};

    let data = TempDir::new()?;
    let ffm = data.path().join("ffm");
    fs::create_dir_all(&ffm)?;
    for name in DATA_FILES {
        crate::test_support::make_fifo(&ffm.join(name))?;
    }
    let _xdg = crate::test_support::EnvVarGuard::set("XDG_DATA_HOME", data.path());
    let session = crate::history::Persistence::File(ffm.join("panel-session.json"));
    let loaded = crate::test_support::run_with_timeout(GUARD_TIMEOUT, move || {
        [
            crate::history::History::load().suggest("cd").is_none(),
            Bookmarks::load().slots().iter().all(Option::is_none),
            FolderHistory::load().is_empty(),
            ViewHistory::load().is_empty(),
            crate::panel_session::PanelSession::load(session)
                .paths()
                .is_none(),
        ]
    })
    .expect("no data file is waited on");
    assert_eq!(loaded, [true; 5]);
    Ok(())
}
