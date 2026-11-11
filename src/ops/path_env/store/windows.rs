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
                super::NEEDS_ADMINISTRATOR
            } else {
                ""
            };
        io::Error::new(
            error.kind(),
            format!("{}: {error}{rights}", self.location()),
        )
    }
}

#[cfg(test)]
#[derive(Clone)]
enum FakeUser {
    Disabled,
    Active(Option<platform::RegistryString>),
}

#[cfg(test)]
thread_local! {
    static FAKE_USER: std::cell::RefCell<FakeUser> =
        const { std::cell::RefCell::new(FakeUser::Disabled) };
}

#[cfg(test)]
pub fn with_fake_user<T>(initial: Option<platform::RegistryString>, run: impl FnOnce() -> T) -> T {
    FAKE_USER.with(|slot| *slot.borrow_mut() = FakeUser::Active(initial));
    let out = run();
    FAKE_USER.with(|slot| *slot.borrow_mut() = FakeUser::Disabled);
    out
}

pub fn read(scope: Scope) -> io::Result<Snapshot> {
    let place = Place::of(scope);
    #[cfg(test)]
    if scope == Scope::User {
        if let FakeUser::Active(fake) = FAKE_USER.with(|slot| slot.borrow().clone()) {
            let value = stored(fake);
            return Ok(Snapshot {
                revision: revision(&value),
                value: value.text,
                location: place.location(),
            });
        }
    }
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
    write_with(
        scope,
        previous,
        value,
        |hive, subkey, name, update| {
            #[cfg(test)]
            if let (Scope::User, FakeUser::Active(current)) =
                (scope, FAKE_USER.with(|slot| slot.borrow().clone()))
            {
                let next = update(current)?;
                FAKE_USER.with(|slot| *slot.borrow_mut() = FakeUser::Active(Some(next)));
                return Ok(());
            }
            platform::update_registry_string(hive, subkey, name, update)
        },
        platform::broadcast_environment_change,
    )
}

fn write_with(
    scope: Scope,
    previous: &Snapshot,
    value: &str,
    update_registry: impl FnOnce(
        RegistryHive,
        &str,
        &str,
        &mut dyn FnMut(Option<platform::RegistryString>) -> io::Result<platform::RegistryString>,
    ) -> io::Result<()>,
    broadcast: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let place = Place::of(scope);
    let mut unchanged = |current| {
        let current = stored(current);
        if revision(&current) == previous.revision {
            Ok(replacement(&current, value))
        } else {
            Err(io::Error::other(CHANGED_OUTSIDE))
        }
    };
    update_registry(place.hive, place.subkey, VALUE, &mut unchanged)
        .map_err(|error| place.failed(&error))?;
    // PATH is saved either way; a program that missed the news sees it in a
    // new session, which the editor's message already asks for.
    if let Err(error) = broadcast() {
        crate::diagnostics::log_error(&format!(
            "PATH saved, but telling running programs failed: {error}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
