//! Purpose: The settings file on disk — one YAML mapping shared by the typed
//! settings and the Options pages, where a save writes only what its writer
//! changed.
//!
//! Two writers own disjoint keys of the one file: [`super::Settings`] owns its
//! pages (`lsp`, `system`, …) and the Options menu owns `options`.
//!
//! **One parse.** The file is parsed once into a YAML value, and each writer
//! reads its settings from that value, never from the text. A save writes the
//! same value back, so what a load sees is exactly what survives any save:
//! read from the text, `Masks: ~` was the text `~` until the other writer's
//! save turned it into `null`. YAML tags (`!x`) are refused, because no setting
//! uses one and YAML reads `Masks: !*.tmp` as a tag on an empty value.
//!
//! **Three-way save.** A writer keeps what it loaded, and a save compares, entry
//! by entry, the writer's settings as loaded, as it holds them now, and as
//! the file holds them now (see [`merge`]). What the session did not change
//! keeps the file's value, hand edits included; what it changed is written;
//! an entry changed both in the session and in the file refuses the save. A
//! save that changes nothing leaves the file alone. An exclusive lock on
//! `settings.yaml.lock` beside the file spans the read, the merge and the
//! rename, so two FFMs saving at once cannot lose each other's changes.
//!
//! Builds before YAML wrote `settings.json`, with the Options pages beside the
//! typed keys. While there is no `settings.yaml`, that file is read in its
//! place with the pages moved under `options`, and the first save writes
//! `settings.yaml`. `settings.json` is never written or removed.
//!
//! A save never replaces a file it cannot read back: when the stored file
//! does not parse — as YAML, or as the saving writer's own keys — the save
//! fails with the reason and the file stays exactly as the user left it.

use std::{
    borrow::Cow,
    ffi::OsString,
    fmt::Display,
    fs::{self, File, TryLockError},
    io,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_norway::{Mapping, Value};

mod merge;

/// The settings file's name under the `ffm` config directory.
pub const FILE_NAME: &str = "settings.yaml";

/// The file builds before YAML wrote, read while [`FILE_NAME`] is absent.
const LEGACY_FILE_NAME: &str = "settings.json";

/// Appended to the settings file's path to name the lock a save holds.
const LOCK_SUFFIX: &str = ".lock";

/// The first line of every saved file. A save rewrites the whole file from
/// the settings, so it says up front that comments do not survive one.
const HEADER: &str =
    "# FunnyFileManager settings. FFM rewrites this file on save; comments are not kept.\n";

/// How every error of a save begins: the status line is half the screen
/// wide, and the outcome must survive its truncation.
const NOT_SAVED: &str = "Not saved: ";

/// Largest settings file or language dictionary read, in bytes.
const MAX_FILE_BYTES: u64 = 1_048_576;

/// The syntax a stored settings file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Yaml,
    /// The `settings.json` of builds before YAML.
    LegacyJson,
}

/// A settings file as read from disk: its top-level mapping, in the layout
/// of `settings.yaml` whichever file it came from.
#[derive(Debug)]
pub struct Stored {
    path: PathBuf,
    format: Format,
    document: Mapping,
}

impl Stored {
    /// The stored settings as `T`. The error names the file and the setting:
    /// `settings.yaml does not parse: system.history_size: invalid type: …`.
    pub fn parse<T: DeserializeOwned>(&self) -> io::Result<T> {
        serde_path_to_error::deserialize(Value::Mapping(self.document.clone()))
            .map_err(|error| does_not_parse(&self.path, error))
    }
}

/// The stored settings for `path`: that file, else the legacy JSON file
/// beside it, else `None` when neither exists.
pub fn read(path: &Path) -> io::Result<Option<Stored>> {
    match read_as(path, Format::Yaml)? {
        Some(stored) => Ok(Some(stored)),
        None => read_as(&path.with_file_name(LEGACY_FILE_NAME), Format::LegacyJson),
    }
}

/// `path` read as `format`, or `None` when there is no such file.
fn read_as(path: &Path, format: Format) -> io::Result<Option<Stored>> {
    let text = match read_small_file(path, &file_name(path)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let document = parse_document(&text, format).map_err(|error| does_not_parse(path, error))?;
    Ok(Some(Stored {
        path: path.to_owned(),
        format,
        document,
    }))
}

/// The mapping of settings `text` holds. An empty or comment-only file holds
/// none; a legacy file's pages move under `options`.
fn parse_document(text: &str, format: Format) -> Result<Mapping, String> {
    let value: Value = match format {
        Format::Yaml => serde_norway::from_str(text).map_err(|error| error.to_string())?,
        Format::LegacyJson => serde_json::from_str(text).map_err(|error| error.to_string())?,
    };
    let document = match value {
        Value::Mapping(document) => document,
        Value::Null => Mapping::new(),
        _ => return Err("expected a mapping of settings".to_owned()),
    };
    if let Some(tag) = tag_in_entries(&document) {
        return Err(format!(
            "{tag} is a YAML tag, which no setting uses; \
             put a value that begins with ! in single quotes"
        ));
    }
    Ok(match format {
        Format::Yaml => document,
        Format::LegacyJson => crate::models::settings::nest_legacy_pages(document),
    })
}

/// The first YAML tag among `entries`, after the path to it:
/// `options.Masks: !*.tmp`.
fn tag_in_entries(entries: &Mapping) -> Option<String> {
    entries
        .iter()
        .find_map(|(key, value)| Some(format!("{}{}", key.as_str().unwrap_or("?"), tag_in(value)?)))
}

/// The first YAML tag in `value`, after the rest of the path to it.
fn tag_in(value: &Value) -> Option<String> {
    match value {
        Value::Tagged(tagged) => Some(format!(": {}", tagged.tag)),
        Value::Mapping(entries) => tag_in_entries(entries).map(|found| format!(".{found}")),
        Value::Sequence(items) => items
            .iter()
            .enumerate()
            .find_map(|(index, item)| Some(format!("[{index}]{}", tag_in(item)?))),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

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
    locked(path, || save_changes(path, loaded, current))
        .map_err(|error| io::Error::new(error.kind(), format!("{NOT_SAVED}{error}")))
}

/// Run `save` while holding the lock beside `path`, which every FFM takes
/// before it reads the file to save it.
fn locked(path: &Path, save: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|error| named(dir.display(), &error))?;
    }
    let mut lock_path = OsString::from(path);
    lock_path.push(LOCK_SUFFIX);
    let lock_path = PathBuf::from(lock_path);
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|error| named(file_name(&lock_path), &error))?;
    lock.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("another FFM is saving {}", file_name(path)),
        ),
        TryLockError::Error(error) => named(file_name(&lock_path), &error),
    })?;
    save()
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
    let on_disk = to_mapping(&stored.parse::<T>()?)?;
    let sides = merge::Sides {
        on_disk: &on_disk,
        loaded: &to_mapping(loaded)?,
        current: &current,
    };
    let mut document = stored.document.clone();
    merge::merge(&mut document, &sides).map_err(|entry| {
        io::Error::other(format!(
            "{} changed on disk: {entry} was edited there and in this session",
            file_name(path)
        ))
    })?;
    if stored.format == Format::Yaml && document == stored.document {
        return Ok(());
    }
    write(path, &document)
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

/// Write `document` to `path` under the header, atomically and readable by
/// the owner only: a crash mid-write leaves the previous file intact.
fn write(path: &Path, document: &Mapping) -> io::Result<()> {
    let body = serde_norway::to_string(document).map_err(io::Error::other)?;
    crate::file_write::ensure_writable(path)?;
    crate::history::write_owner_only(path, &format!("{HEADER}{body}"))
        .map_err(|error| named(file_name(path), &error))
}

/// The text of a small configuration file — the settings, or a language
/// dictionary — with `what` naming it in every error.
///
/// Anything but a regular file is refused before it is opened: opening a FIFO
/// blocks until something writes to it, and this runs on the event loop, so
/// the whole UI froze. A FIFO or a device also reports a length of 0, which
/// the size check alone lets through.
pub fn read_small_file(path: &Path, what: &str) -> io::Result<String> {
    let metadata = fs::metadata(path).map_err(|error| named(what, &error))?;
    if !metadata.is_file() {
        return Err(io::Error::other(format!("{what} is not a regular file")));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(io::Error::other(format!("{what} exceeds 1 MiB")));
    }
    fs::read_to_string(path).map_err(|error| named(what, &error))
}

/// A file's name for messages; the directory is the user's own.
fn file_name(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
}

/// `error`, of the same kind, after the name of what it happened to.
fn named(what: impl Display, error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{what}: {error}"))
}

/// The error for a settings file whose text or values do not parse.
fn does_not_parse(path: &Path, error: impl Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{} does not parse: {error}", file_name(path)),
    )
}

#[cfg(test)]
mod tests;
