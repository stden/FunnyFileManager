//! Purpose: A save under the settings lock — the three-way merge of a typed
//! writer's changes ([`update`]) or an edit of the file's own mapping
//! ([`edit`]), both through [`rewrite`], which writes only a file that loads.

use std::{
    ffi::OsString,
    fs::{File, TryLockError},
    io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_norway::{Mapping, Value};

use super::{
    file_name, merge, named, parse::parse_mapping, read, write::write, Format, Stored, LOCK_SUFFIX,
    NOT_SAVED,
};

/// How long a save waits for the lock before it says another FFM holds it.
/// A program this FFM starts on another thread shares every open file, the
/// lock among them, from its fork until its exec, so a lock this FFM has
/// just let go of can read as held for that moment; another FFM's save
/// takes milliseconds too.
const LOCK_PATIENCE: Duration = Duration::from_millis(250);

/// How often the lock is tried while waiting for it.
const LOCK_RETRY: Duration = Duration::from_millis(10);

/// Save a writer's settings into the file at `path`: what changed between
/// `loaded` — the settings as the writer loaded them, or its defaults — and
/// `current` is written, and every other entry keeps the file's value.
///
/// Nothing is written when the stored file cannot be read, does not parse
/// both as YAML and as `T`, or changed an entry the session changed too. A
/// writer whose load failed holds its defaults, and saving those would
/// replace what the user wrote by hand; a syntax error would take the other
/// writer's keys along with it. Every error begins with [`NOT_SAVED`].
pub fn update<T: Serialize + DeserializeOwned>(
    path: &Path,
    loaded: &T,
    current: &T,
) -> io::Result<()> {
    not_saved(locked(path, || save_changes(path, loaded, current)))
}

/// Change the stored settings with `change` and write them back, under the
/// lock every save takes.
///
/// `change` edits the file's own mapping in place, and gets the file's
/// settings as `T` to check the edit against; its error is why the save is
/// refused. With no stored file the mapping is empty and `T` its defaults.
/// Nothing is written when the stored file cannot be read or does not parse
/// (as YAML and as `T`), when `change` refuses, when the edited mapping would
/// not parse as `T`, or when it changed nothing. Returns the settings the
/// file holds afterwards. Every error begins with [`NOT_SAVED`].
pub fn edit<T: DeserializeOwned>(
    path: &Path,
    change: impl FnOnce(&mut Mapping, &T) -> io::Result<()>,
) -> io::Result<T> {
    not_saved(locked(path, || {
        let stored = read(path)?.unwrap_or_else(|| Stored::empty(path));
        rewrite(path, &stored, change)
    }))
}

/// `result`, its error prefixed with [`NOT_SAVED`].
fn not_saved<R>(result: io::Result<R>) -> io::Result<R> {
    result.map_err(|error| io::Error::new(error.kind(), format!("{NOT_SAVED}{error}")))
}

/// Run `save` while holding the lock beside `path`, which every FFM takes
/// before it reads the file to save it.
fn locked<R>(path: &Path, save: impl FnOnce() -> io::Result<R>) -> io::Result<R> {
    if let Some(dir) = path.parent() {
        // Named by its last part, as the files are: the whole path in front
        // of the reason left no room for the reason on the status line.
        crate::file_write::create_dirs_owner_only(dir)
            .map_err(|error| named(format!("cannot create folder {}", file_name(dir)), &error))?;
    }
    let mut lock_path = OsString::from(path);
    lock_path.push(LOCK_SUFFIX);
    let lock_path = PathBuf::from(lock_path);
    // A FIFO in the lock's place is refused, never waited on: every save and
    // the one at exit take this lock on the event loop.
    let lock = crate::fileload::open_regular(
        &lock_path,
        File::options().create(true).truncate(false).write(true),
    )
    .map_err(|error| named(file_name(&lock_path), &error))?;
    take_lock(&lock, path, &lock_path)?;
    save()
}

/// Hold `lock`, waiting up to [`LOCK_PATIENCE`] for whoever holds it now.
fn take_lock(lock: &File, path: &Path, lock_path: &Path) -> io::Result<()> {
    let deadline = Instant::now() + LOCK_PATIENCE;
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => thread::sleep(LOCK_RETRY),
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!("another FFM is saving {}", file_name(path)),
                ))
            }
            Err(TryLockError::Error(error)) => return Err(named(file_name(lock_path), &error)),
        }
    }
}

/// Merge `current` into the stored file and write the result, unless it
/// changes nothing. With no stored file, `current` is the whole file.
fn save_changes<T: Serialize + DeserializeOwned>(
    path: &Path,
    loaded: &T,
    current: &T,
) -> io::Result<()> {
    let current = to_mapping(current)?;
    let Some(stored) = read(path)? else {
        return write(path, &current);
    };
    let loaded = to_mapping(loaded)?;
    rewrite(path, &stored, |document, on_disk: &T| {
        let sides = merge::Sides {
            on_disk: &to_mapping(on_disk)?,
            loaded: &loaded,
            current: &current,
        };
        merge::merge(document, &sides).map_err(|entry| {
            io::Error::other(format!(
                "{} changed on disk: {entry} was edited there and in this session",
                file_name(path)
            ))
        })
    })?;
    Ok(())
}

/// What every save does once the stored file is read: parse it as `T`, let
/// `change` edit a copy, check that the copy still parses as `T`, and write
/// it to `path` unless it is the YAML file unchanged or the file changed
/// since `stored` was read. Returns the copy as `T`.
fn rewrite<T: DeserializeOwned>(
    path: &Path,
    stored: &Stored,
    change: impl FnOnce(&mut Mapping, &T) -> io::Result<()>,
) -> io::Result<T> {
    let on_disk: T = stored.parse()?;
    let mut document = stored.document.clone();
    change(&mut document, &on_disk)?;
    let saved = parse_mapping(&document).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the change would not parse: {error}"),
        )
    })?;
    if stored.format == Format::Yaml && document == stored.document {
        return Ok(saved);
    }
    refuse_changed_since(stored, path)?;
    write(path, &document)?;
    Ok(saved)
}

/// Refuse the save when the stored settings for `path` are no longer
/// `stored`. The lock keeps other FFMs out, but an editor, `sed -i` or a
/// dotfiles sync takes none, and writing over its change would lose it.
fn refuse_changed_since(stored: &Stored, path: &Path) -> io::Result<()> {
    if read(path)?.unwrap_or_else(|| Stored::empty(path)) == *stored {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "{} changed while it was being saved; save again",
        file_name(path)
    )))
}

/// `owned` as a YAML mapping: its keys are the ones its writer owns.
fn to_mapping(owned: &impl Serialize) -> io::Result<Mapping> {
    match serde_norway::to_value(owned).map_err(io::Error::other)? {
        Value::Mapping(mapping) => Ok(mapping),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "settings must be a mapping of named values",
        )),
    }
}
