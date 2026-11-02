//! Purpose: An open registry key and the raw Win32 calls made on it: open,
//! create, read one value, set one string, close.

use std::ffi::OsStr;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegFlushKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE,
};

use super::{check, nul_terminated, os_error, RegistryHive};
use crate::platform::registry_text::{encode, RegistryString, RegistryStringKind};

/// An open registry key, closed when dropped.
pub(super) struct Key(HKEY);

impl Key {
    /// `subkey` of `hive`, opened to read its values; `None` when it does
    /// not exist.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    pub(super) fn open(hive: RegistryHive, subkey: &str) -> io::Result<Option<Self>> {
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
    pub(super) fn create(hive: RegistryHive, subkey: &str) -> io::Result<Self> {
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

    /// One read of the value `name` (NUL-terminated) into `units`; an empty
    /// `units` asks for the value's type and size only.
    #[allow(
        unsafe_code,
        reason = "std has no registry API, and PowerShell took seconds to start"
    )]
    pub(super) fn query_into(&self, name: &[u16], units: &mut [u16]) -> io::Result<Answer> {
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
    pub(super) fn set_string(&self, name: &[u16], value: &RegistryString) -> io::Result<()> {
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
pub(super) struct Answer {
    pub(super) status: WIN32_ERROR,
    pub(super) kind: REG_VALUE_TYPE,
    pub(super) bytes: u32,
}

/// The byte length of `units`, as the registry API counts it.
fn byte_len(units: &[u16]) -> io::Result<u32> {
    u32::try_from(size_of_val(units))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}
