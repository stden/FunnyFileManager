//! Purpose: Read a client's config safely — a bounded regular file of UTF-8
//! text — and say what it holds about the server entry.

use std::{io, path::Path};

use crate::fileload::{self, Loaded};
use crate::mcp_clients::{self, Blocker, ConfigFormat, ConfigState, ServerEntry};

/// The most of a config that is read. Claude Code's `.claude.json` keeps a
/// history for every project and runs to megabytes; a config past this is
/// not one to edit blind.
pub(super) const MAX_CONFIG_BYTES: u64 = 64 * 1024 * 1024;

/// A config's text, `None` when there is no file — including when a folder
/// on the way to it is a file, which the write then reports.
///
/// Only a regular file of at most [`MAX_CONFIG_BYTES`] is read: opening a
/// FIFO named `.mcp.json` would block the dialog forever, and a link to
/// `/dev/zero` would fill the memory (see [`fileload::read_regular`]).
pub fn load(path: &Path) -> Result<Option<String>, Blocker> {
    match fileload::read_regular(path, MAX_CONFIG_BYTES) {
        Ok(Loaded::Bytes(bytes)) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| Blocker::Unreadable(format!("not UTF-8 text ({error})"))),
        Ok(Loaded::NotRegular) => Err(Blocker::Unreadable("not a regular file".to_owned())),
        Ok(Loaded::TooLarge(size)) => Err(Blocker::Unreadable(format!(
            "{size} bytes, more than the {MAX_CONFIG_BYTES} a config may have"
        ))),
        Err(error) => match error.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Ok(None),
            _ => Err(Blocker::Unreadable(error.to_string())),
        },
    }
}

/// What the config at `path` says about `desired`.
#[must_use]
pub fn read_state(path: &Path, format: ConfigFormat, desired: Option<&ServerEntry>) -> ConfigState {
    match load(path) {
        Ok(None) => ConfigState::NoConfig,
        Ok(Some(text)) => check_program(mcp_clients::state(format, &text, desired)),
        Err(blocker) => ConfigState::Blocked(blocker),
    }
}

/// An entry nothing could be compared with still names what it runs: an
/// absolute path with nothing behind it is an entry that cannot start. A bare
/// name is the client's PATH's business, which FFM cannot see.
fn check_program(state: ConfigState) -> ConfigState {
    match state {
        ConfigState::Unverified {
            command: Some(command),
        } if Path::new(&command).is_absolute() && !Path::new(&command).exists() => {
            ConfigState::ProgramMissing
        }
        other => other,
    }
}
