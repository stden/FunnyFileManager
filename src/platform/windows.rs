//! Purpose: The raw Win32 calls behind the Windows platform wrappers: the
//! mounted-drive mask, a file's attribute bits, registry string values, and
//! the broadcast that tells running programs the environment changed.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR,
};
use windows_sys::Win32::Storage::FileSystem::{GetLogicalDrives, SetFileAttributesW};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegFlushKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ,
    REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

use super::registry_text::{decode, encode, RegistryString, RegistryStringKind};

/// The drive letters Windows has mounted, as `GetLogicalDrives`' bitmask:
/// bit 0 is `A:`, bit 25 is `Z:`. Zero means the call failed.
#[allow(
    unsafe_code,
    reason = "std has no call listing the mounted drive letters"
)]
#[must_use]
pub fn logical_drives() -> u32 {
    // SAFETY: `GetLogicalDrives` takes no arguments, reads no caller memory
    // and only returns a bitmask.
    unsafe { GetLogicalDrives() }
}

/// Replace `path`'s attribute bits (`FILE_ATTRIBUTE_HIDDEN`, `…_READONLY`, …).
///
/// # Errors
///
/// `InvalidInput` for a path with an interior NUL — Win32 would silently act
/// on the part before it — and otherwise the OS error of the failed call.
#[allow(
    unsafe_code,
    reason = "std sets only the read-only bit; hidden, system and archive need Win32"
)]
pub fn set_file_attributes(path: &Path, attributes: u32) -> io::Result<()> {
    let wide = nul_terminated(path.as_os_str())?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call,
    // and the function reads nothing else.
    if unsafe { SetFileAttributesW(wide.as_ptr(), attributes) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `text` as the NUL-terminated UTF-16 a Win32 call reads.
///
/// # Errors
///
/// `InvalidInput` for text with an interior NUL: Win32 would silently act on
/// the part before it.
fn nul_terminated(text: &OsStr) -> io::Result<Vec<u16>> {
    let mut wide: Vec<u16> = text.encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a name contains a NUL character",
        ));
    }
    wide.push(0);
    Ok(wide)
}

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

/// How long one window may take to answer the broadcast.
const BROADCAST_TIMEOUT_MS: u32 = 1000;

/// Tell every top-level window that the environment changed, as `setx` and
/// the System Properties dialog do, so Explorer starts new programs with it.
/// A window that hangs is skipped; one that is slow gets a second.
///
/// # Errors
///
/// The OS error when the broadcast fails or times out.
#[allow(
    unsafe_code,
    reason = "std cannot send a window message; WM_SETTINGCHANGE is how Windows announces a new environment"
)]
pub fn broadcast_environment_change() -> io::Result<()> {
    let environment = nul_terminated(OsStr::new("Environment"))?;
    // The message carries the section's name as a pointer in its LPARAM.
    let section = isize::try_from(environment.as_ptr().expose_provenance())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut answer: usize = 0;
    // SAFETY: `section` points at `environment`, NUL-terminated UTF-16 that
    // outlives the call, which only reads it; `answer` is a valid place for
    // the result.
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            section,
            SMTO_ABORTIFHUNG,
            BROADCAST_TIMEOUT_MS,
            &raw mut answer,
        )
    };
    if sent == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// How many times a value that grows between asking its size and reading it
/// is asked again before the read gives up.
const READ_ATTEMPTS: usize = 4;

/// An open registry key, closed when dropped.
struct Key(HKEY);

impl Key {
    /// `subkey` of `hive`, opened to read its values; `None` when it does
    /// not exist.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    fn open(hive: RegistryHive, subkey: &str) -> io::Result<Option<Self>> {
        let subkey = nul_terminated(OsStr::new(subkey))?;
        let mut handle: HKEY = ptr::null_mut();
        // SAFETY: `subkey` is NUL-terminated UTF-16 that outlives the call,
        // and `handle` is a valid place for the opened key.
        let status = unsafe {
            RegOpenKeyExW(
                hive.handle(),
                subkey.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &raw mut handle,
            )
        };
        match status {
            ERROR_SUCCESS => Ok(Some(Self(handle))),
            ERROR_FILE_NOT_FOUND => Ok(None),
            error => Err(os_error(error)),
        }
    }

    /// `subkey` of `hive`, created when missing, opened to read and set its
    /// values.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    fn create(hive: RegistryHive, subkey: &str) -> io::Result<Self> {
        let subkey = nul_terminated(OsStr::new(subkey))?;
        let mut handle: HKEY = ptr::null_mut();
        // SAFETY: `subkey` is NUL-terminated UTF-16 that outlives the call;
        // no class and no security descriptor are passed, which the call
        // allows; `handle` is a valid place for the key, and the disposition
        // is not asked for.
        let status = unsafe {
            RegCreateKeyExW(
                hive.handle(),
                subkey.as_ptr(),
                0,
                ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_QUERY_VALUE | KEY_SET_VALUE,
                ptr::null(),
                &raw mut handle,
                ptr::null_mut(),
            )
        };
        check(status)?;
        Ok(Self(handle))
    }

    /// The string value `name` (NUL-terminated), or `None` when there is none.
    fn string(&self, name: &[u16]) -> io::Result<Option<RegistryString>> {
        let Some((kind, units)) = self.query(name)? else {
            return Ok(None);
        };
        let kind = match kind {
            REG_SZ => RegistryStringKind::Plain,
            REG_EXPAND_SZ => RegistryStringKind::Expandable,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("the value is not a string (registry type {other})"),
                ))
            }
        };
        Ok(Some(RegistryString {
            kind,
            text: decode(&units)?,
        }))
    }

    /// The type and the data of the value `name` (NUL-terminated), or `None`
    /// when there is none. The data is read as UTF-16 units, the form of a
    /// string value; an odd last byte is dropped.
    fn query(&self, name: &[u16]) -> io::Result<Option<(REG_VALUE_TYPE, Vec<u16>)>> {
        let mut units: Vec<u16> = Vec::new();
        for _ in 0..READ_ATTEMPTS {
            let answer = self.query_into(name, &mut units)?;
            // An empty buffer asked for the size only.
            let fetched = !units.is_empty() || answer.bytes == 0;
            match answer.status {
                ERROR_SUCCESS if fetched => {
                    units.truncate(units_in(answer.bytes / 2)?);
                    return Ok(Some((answer.kind, units)));
                }
                ERROR_SUCCESS | ERROR_MORE_DATA => {
                    units.resize(units_in(answer.bytes.div_ceil(2))?, 0);
                }
                ERROR_FILE_NOT_FOUND => return Ok(None),
                error => return Err(os_error(error)),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "the value kept changing while it was read",
        ))
    }

    /// One read of the value `name` (NUL-terminated) into `units`; an empty
    /// `units` asks for the value's type and size only.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    fn query_into(&self, name: &[u16], units: &mut [u16]) -> io::Result<Answer> {
        let mut kind: REG_VALUE_TYPE = 0;
        let mut bytes = byte_len(units)?;
        let data = if units.is_empty() {
            ptr::null_mut()
        } else {
            units.as_mut_ptr().cast::<u8>()
        };
        // SAFETY: `name` is NUL-terminated UTF-16 that outlives the call;
        // `data` is null or `units`, which is `bytes` long; `kind` and
        // `bytes` are valid places for the type and the length.
        let status = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                ptr::null(),
                &raw mut kind,
                data,
                &raw mut bytes,
            )
        };
        Ok(Answer {
            status,
            kind,
            bytes,
        })
    }

    /// Set the value `name` (NUL-terminated) to `value`, and write the key
    /// to disk before returning.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    fn set_string(&self, name: &[u16], value: &RegistryString) -> io::Result<()> {
        let data = encode(&value.text)?;
        let bytes = byte_len(&data)?;
        let kind = match value.kind {
            RegistryStringKind::Plain => REG_SZ,
            RegistryStringKind::Expandable => REG_EXPAND_SZ,
        };
        // SAFETY: `name` is NUL-terminated UTF-16 and `data` is `bytes`
        // long; both outlive the call, which only reads them.
        check(unsafe {
            RegSetValueExW(
                self.0,
                name.as_ptr(),
                0,
                kind,
                data.as_ptr().cast::<u8>(),
                bytes,
            )
        })?;
        // SAFETY: `self.0` is a key this value opened and has not closed.
        check(unsafe { RegFlushKey(self.0) })
    }
}

impl Drop for Key {
    #[allow(
        unsafe_code,
        reason = "the key is a raw registry handle, closed by the registry API"
    )]
    fn drop(&mut self) {
        // Only a handle that is not a key fails to close, and nothing is
        // left to do with it.
        // SAFETY: `self.0` is a key this value opened, closed here once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

/// What one `RegQueryValueExW` said: its status, the value's type, and the
/// length of its data in bytes — all of it, when the buffer was too small.
struct Answer {
    status: WIN32_ERROR,
    kind: REG_VALUE_TYPE,
    bytes: u32,
}

/// The byte length of `units`, as the registry API counts it.
fn byte_len(units: &[u16]) -> io::Result<u32> {
    u32::try_from(size_of_val(units))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

/// A count of UTF-16 units the registry API gave, as a length.
fn units_in(count: u32) -> io::Result<usize> {
    usize::try_from(count).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
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
