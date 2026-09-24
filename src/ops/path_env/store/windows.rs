//! Purpose: The Windows PATH store: the current user's and the machine's
//! `Path` values in the registry, read and written through the registry API.
//!
//! PowerShell used to do this. Its start took seconds on a cold machine, on
//! the UI thread, before the PATH editor could open; a registry read takes
//! microseconds. After a save, running programs are told the environment
//! changed, as `setx` tells them.

use std::io;

use super::registry::{replacement, revision, stored};
use super::CHANGED_OUTSIDE;
use crate::path_env::{Scope, Snapshot};
use crate::platform::{self, RegistryHive};

/// The value both scopes keep PATH in.
const VALUE: &str = "Path";

/// Where a scope's PATH lives.
struct Place {
    scope: Scope,
    hive: RegistryHive,
    subkey: &'static str,
}

impl Place {
    const fn of(scope: Scope) -> Self {
        let (hive, subkey) = match scope {
            Scope::User => (RegistryHive::CurrentUser, "Environment"),
            Scope::System => (
                RegistryHive::LocalMachine,
                r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            ),
        };
        Self {
            scope,
            hive,
            subkey,
        }
    }

    /// The key as the editor names it on its first row.
    fn location(&self) -> String {
        format!(r"{}\{}", self.hive.label(), self.subkey)
    }

    /// `error` naming the key it happened at, and saying so when it is the
    /// administrator rights the machine's PATH needs.
    fn failed(&self, error: &io::Error) -> io::Error {
        let rights =
            if self.scope == Scope::System && error.kind() == io::ErrorKind::PermissionDenied {
                "; the PATH of all users needs administrator rights"
            } else {
                ""
            };
        io::Error::new(
            error.kind(),
            format!("{}: {error}{rights}", self.location()),
        )
    }
}

pub fn read(scope: Scope) -> io::Result<Snapshot> {
    let place = Place::of(scope);
    let value = platform::read_registry_string(place.hive, place.subkey, VALUE)
        .map(stored)
        .map_err(|error| place.failed(&error))?;
    Ok(Snapshot {
        revision: revision(&value),
        value: value.text,
        location: place.location(),
    })
}

/// Replace `scope`'s PATH with `value`, unless it changed since `previous`
/// was read; then tell running programs.
pub fn write(scope: Scope, previous: &Snapshot, value: &str) -> io::Result<()> {
    let place = Place::of(scope);
    let unchanged = |current| {
        let current = stored(current);
        if revision(&current) == previous.revision {
            Ok(replacement(&current, value))
        } else {
            Err(io::Error::other(CHANGED_OUTSIDE))
        }
    };
    platform::update_registry_string(place.hive, place.subkey, VALUE, unchanged)
        .map_err(|error| place.failed(&error))?;
    // PATH is saved either way; a program that missed the news sees it in a
    // new session, which the editor's message already asks for.
    if let Err(error) = platform::broadcast_environment_change() {
        crate::diagnostics::log_error(&format!(
            "PATH saved, but telling running programs failed: {error}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
