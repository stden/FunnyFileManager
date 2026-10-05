//! Purpose: Registry string values by hive, key and name: the reads and the
//! read-then-write updates the PATH editor makes.

use std::ffi::OsStr;
use std::io;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, WIN32_ERROR};
use windows_sys::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use super::nul_terminated;
use crate::platform::registry_text::RegistryString;

mod key;
mod value;

use key::Key;

/// A registry root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryHive {
    /// `HKEY_CURRENT_USER`: this user's settings.
    CurrentUser,
    /// `HKEY_LOCAL_MACHINE`: the machine's, for every user.
    LocalMachine,
}

impl RegistryHive {
    /// The registry's name for the root.
    pub const fn label(self) -> &'static str {
        match self {
            Self::CurrentUser => "HKEY_CURRENT_USER",
            Self::LocalMachine => "HKEY_LOCAL_MACHINE",
        }
    }

    const fn handle(self) -> HKEY {
        match self {
            Self::CurrentUser => HKEY_CURRENT_USER,
            Self::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }
}

/// The string value `name` of the key `hive\subkey`, or `None` when the key
/// or the value does not exist.
///
/// # Errors
///
/// The OS error of a key or value that cannot be read, and `InvalidData`
/// for a value that is not a string or whose text is not UTF-16.
pub fn read_registry_string(
    hive: RegistryHive,
    subkey: &str,
    name: &str,
) -> io::Result<Option<RegistryString>> {
    let Some(key) = Key::open(hive, subkey)? else {
        return Ok(None);
    };
    key.string(&nul_terminated(OsStr::new(name))?)
}

/// Set the string value `name` of the key `hive\subkey`, created when
/// missing, to what `decide` makes of the value it holds now.
///
/// The value is read and written through one open handle, so `decide` sees
/// what is replaced; its error leaves the value as it was.
///
/// # Errors
///
/// `decide`'s error, the OS error of a key that cannot be opened for
/// writing (`PermissionDenied` for `HKEY_LOCAL_MACHINE` without
/// administrator rights), and [`read_registry_string`]'s.
pub fn update_registry_string(
    hive: RegistryHive,
    subkey: &str,
    name: &str,
    decide: impl FnOnce(Option<RegistryString>) -> io::Result<RegistryString>,
) -> io::Result<()> {
    let key = Key::create(hive, subkey)?;
    let name = nul_terminated(OsStr::new(name))?;
    let value = decide(key.string(&name)?)?;
    key.set_string(&name, &value)
}

/// `Ok` for `ERROR_SUCCESS`, else the OS error.
fn check(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(os_error(status))
    }
}

/// A registry call's status as an `io::Error`.
fn os_error(status: WIN32_ERROR) -> io::Error {
    i32::try_from(status).map_or_else(
        |_| io::Error::other(format!("registry error {status}")),
        io::Error::from_raw_os_error,
    )
}
