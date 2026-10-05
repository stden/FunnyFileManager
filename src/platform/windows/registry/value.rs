//! Purpose: Reading one string value off an open key: asking its size,
//! reading again while it grows, and checking that it is a string.

use std::io;

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{REG_EXPAND_SZ, REG_SZ, REG_VALUE_TYPE};

use super::key::Key;
use super::os_error;
use crate::platform::registry_text::{decode, RegistryString, RegistryStringKind};

/// How many times a value that grows between asking its size and reading it
/// is asked again before the read gives up.
const READ_ATTEMPTS: usize = 4;

impl Key {
    /// The string value `name` (NUL-terminated), or `None` when there is none.
    pub(super) fn string(&self, name: &[u16]) -> io::Result<Option<RegistryString>> {
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
}

/// A count of UTF-16 units the registry API gave, as a length.
fn units_in(count: u32) -> io::Result<usize> {
    usize::try_from(count).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
