//! Purpose: Register the server by editing a client's config file — the one
//! path every JSON and TOML client takes — without ever losing what the file
//! held before.
//!
//! The file is read and checked again just before the write, the new text is
//! complete before anything is replaced, the file is compared with what was
//! read once more right before the rename, and the replacement is one atomic
//! rename. A file that does not parse is reported and left alone, and a
//! symlinked dotfile keeps its link. No `.bak` is left behind: beside a
//! project's `.mcp.json` it would land in the user's repository.

use std::{fs, io, path::Path};

use super::report::{Failure, Outcome, Registration};
use super::survey::{self, Target};
use crate::file_write;
use crate::mcp_clients::{self, Client, ConfigState, Host, ServerBinary, ServerEntry};

/// Register `server` in `client`'s config for `target`.
pub fn register(
    host: &Host,
    client: &Client,
    target: Target<'_>,
    server: &ServerBinary,
) -> Registration {
    let path = match survey::config_path(host, client, target) {
        Ok(path) => path,
        Err(blocker) => return Registration::stopped(client.id, None, Failure::Blocked(blocker)),
    };
    let outcome = survey::shared_with_all_projects(host, client, target, &path).map_or_else(
        || update(&path, client, &target.entry(server)),
        |blocker| Outcome::Failed(Failure::Blocked(blocker)),
    );
    Registration {
        client: client.id,
        outcome,
        config: Some(path),
        ran: Vec::new(),
    }
}

/// Set the entry in the file at `path`, or say why not.
fn update(path: &Path, client: &Client, desired: &ServerEntry) -> Outcome {
    let text = match survey::load(path) {
        Ok(text) => text,
        Err(blocker) => return Outcome::Failed(Failure::Blocked(blocker)),
    };
    let state = text.as_deref().map_or(ConfigState::NoConfig, |text| {
        mcp_clients::state(client.format, text, Some(desired))
    });
    let done = match state {
        ConfigState::Registered | ConfigState::Disabled => return Outcome::Unchanged,
        ConfigState::Blocked(blocker) => return Outcome::Failed(Failure::Blocked(blocker)),
        // With a server to compare against, an entry either matches or
        // differs; the other two states cannot come back here.
        ConfigState::DifferentEntry
        | ConfigState::Unverified { .. }
        | ConfigState::ProgramMissing => Outcome::Updated,
        ConfigState::NoConfig | ConfigState::NotConfigured => Outcome::Added,
    };
    let result = mcp_clients::upsert(client.format, text.as_deref().unwrap_or_default(), desired)
        .map_err(Failure::Blocked)
        .and_then(|updated| {
            replace(path, text.as_deref(), &updated).map_err(|error| Failure::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        });
    match result {
        Ok(()) => done,
        Err(failure) => Outcome::Failed(failure),
    }
}

/// Replace the file at `path`, which read as `read` (`None`: absent), with
/// `text` in one rename.
///
/// An existing file is written through its links, so a symlinked dotfile
/// keeps its link and its target changes. A new file gets its folders, and
/// the temp file's owner-only mode.
fn replace(path: &Path, read: Option<&str>, text: &str) -> io::Result<()> {
    let target = if fs::symlink_metadata(path).is_ok() {
        fs::canonicalize(path)?
    } else {
        // A config path always has a parent; "" would mean "here", which
        // `create_dir_all` accepts as already there.
        fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("")))?;
        path.to_path_buf()
    };
    file_write::ensure_writable(&target)?;
    let mut temp = file_write::temp_beside(&target)?;
    file_write::fill(&mut temp, &target, text.as_bytes())?;
    still_as_read(path, read)?;
    temp.persist(&target).map_err(|error| error.error)?;
    Ok(())
}

/// Refuse the rename when the file is no longer what was read: the client
/// saved it meanwhile (Kiro and Junie write theirs from the IDE), and
/// renaming now would throw that save away.
fn still_as_read(path: &Path, read: Option<&str>) -> io::Result<()> {
    let now = survey::load(path).map_err(|blocker| io::Error::other(blocker.explain("it")))?;
    if now.as_deref() == read {
        Ok(())
    } else {
        Err(io::Error::other(
            "it changed while being written, and was left as it now is; register again",
        ))
    }
}

#[cfg(test)]
mod tests;
