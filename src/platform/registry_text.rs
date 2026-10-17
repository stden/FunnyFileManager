//! Purpose: Registry string values as Rust text — the two kinds the file
//! manager reads and writes, and the UTF-16 the registry keeps them in.
//!
//! No system calls: `windows` makes those. Text in, text out, so every
//! platform compiles and tests these rules.

use std::io;

/// How a registry string value is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStringKind {
    /// `REG_SZ`: the text as it is.
    Plain,
    /// `REG_EXPAND_SZ`: a `%NAME%` in the text expands when it is used.
    Expandable,
}

impl RegistryStringKind {
    /// The registry's name for the kind.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Plain => "REG_SZ",
            Self::Expandable => "REG_EXPAND_SZ",
        }
    }
}

/// A registry string value: its kind, and its text without the NUL that
/// ends it in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryString {
    pub kind: RegistryStringKind,
    pub text: String,
}

/// The text of a string value's data, `units`.
///
/// Every NUL that ends the data is dropped. The registry keeps whatever its
/// writer gave it: one terminating NUL, none, or several.
///
/// # Errors
///
/// `InvalidData` when the data is not UTF-16, or holds a NUL before its end:
/// text like that cannot be shown for editing and written back unchanged.
pub(super) fn decode(units: &[u16]) -> io::Result<String> {
    let end = units
        .iter()
        .rposition(|&unit| unit != 0)
        .map_or(0, |last| last + 1);
    let text = units.get(..end).unwrap_or_default();
    if text.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the value holds a NUL character",
        ));
    }
    String::from_utf16(text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// `text` as a string value's data: UTF-16, ended by one NUL.
///
/// # Errors
///
/// `InvalidInput` for text holding a NUL, which would end the value there.
pub(super) fn encode(text: &str) -> io::Result<Vec<u16>> {
    if text.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a registry string cannot hold a NUL character",
        ));
    }
    Ok(text.encode_utf16().chain([0]).collect())
}

#[cfg(test)]
mod tests;
