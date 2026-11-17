//! Filesystem capability checks for fixtures with non-Unicode names.

use std::io;

/// Skip an unsupported filename, while preserving unrelated setup failures.
pub fn non_unicode_name(result: io::Result<()>) -> io::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(error) if error.raw_os_error() == Some(libc::EILSEQ) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
