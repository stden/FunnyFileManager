//! Purpose: The settings file on disk — one YAML mapping shared by the typed
//! settings and the Options pages, where a save writes only what its writer
//! changed.
//!
//! Two writers own disjoint keys of the one file: [`super::Settings`] owns its
//! pages (`lsp`, `system`, …) and the Options menu owns `options`. A third
//! path, [`edit`], changes the file's own mapping in place: the Language
//! servers dialog writes only the `lsp` fields it changed that way.
//!
//! **One parse.** The file is parsed once into a YAML value, and each writer
//! reads its settings from that value, never from the text. A save writes the
//! same value back, so what a load sees is exactly what survives any save:
//! read from the text, `Masks: ~` was the text `~` until the other writer's
//! save turned it into `null`. YAML tags (`!x`) are refused, because no setting
//! uses one and YAML reads `Masks: !*.tmp` as a tag on an empty value. Two
//! tags are read rather than refused: the session origins an earlier build
//! wrote (`!NetBoxFile`, `!Things`).
//!
//! **Three-way save.** A writer keeps what it loaded, and a save compares, entry
//! by entry, the writer's settings as loaded, as it holds them now, and as
//! the file holds them now (see [`merge`]). What the session did not change
//! keeps the file's value, hand edits included; what it changed is written;
//! an entry changed both in the session and in the file refuses the save. A
//! save that changes nothing leaves the file alone. An exclusive lock on
//! `settings.yaml.lock` beside the file spans the read, the merge and the
//! rename, so two FFMs saving at once cannot lose each other's changes. Other
//! programs — an editor, `sed -i`, a dotfiles sync — take no lock, so the file
//! is read again just before the write, and a save is refused when it changed
//! since the first read.
//!
//! **One core.** [`update`] and [`edit`] share [`save::rewrite`]: parse the stored
//! file, change a copy, parse the copy again, and write it unless nothing
//! changed. What reaches the disk is thus always a file that loads.
//!
//! Builds before YAML wrote `settings.json`, with the Options pages beside the
//! typed keys. While there is no `settings.yaml`, that file is read in its
//! place with the pages moved under `options`, and the first save writes
//! `settings.yaml`. `settings.json` is never written or removed.
//!
//! A save never replaces a file it cannot read back: when the stored file
//! does not parse — as YAML, or as the saving writer's own keys — the save
//! fails with the reason and the file stays exactly as the user left it.
//!
//! Nor one a rename would damage: the write goes through a symbolic link to
//! the file it names, and refuses a link that leads nowhere, a file with
//! other hard links, another user's file and a read-only one (see [`write`]).

use std::{
    borrow::Cow,
    fmt::Display,
    io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_norway::Mapping;

mod merge;
mod parse;
mod save;
mod write;

use parse::{parse_document, parse_mapping};
pub use save::{edit, update};

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

/// The syntax a stored settings file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Yaml,
    /// The `settings.json` of builds before YAML.
    LegacyJson,
}

/// A settings file as read from disk: its top-level mapping, in the layout
/// of `settings.yaml` whichever file it came from.
#[derive(Debug, PartialEq, Eq)]
pub struct Stored {
    path: PathBuf,
    format: Format,
    document: Mapping,
}

impl Stored {
    /// No stored file: an empty `settings.yaml` at `path`, which an edit that
    /// adds nothing leaves unwritten.
    fn empty(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            format: Format::Yaml,
            document: Mapping::new(),
        }
    }

    /// The stored settings as `T`. The error names the file and the setting:
    /// `settings.yaml does not parse: system.history_size: invalid type: …`.
    pub fn parse<T: DeserializeOwned>(&self) -> io::Result<T> {
        parse_mapping(&self.document).map_err(|error| does_not_parse(&self.path, error))
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
    let text = match crate::fileload::read_small_file(path, &file_name(path)) {
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
