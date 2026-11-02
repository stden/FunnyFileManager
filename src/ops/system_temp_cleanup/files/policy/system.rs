//! Purpose: Resolve the OS temporary folder without allowing shared Windows/system paths.
use crate::models::temp_cleanup::{TempError, TempSource};

#[cfg(windows)]
pub(super) fn source() -> Result<TempSource, TempError> {
    let home = crate::config::user_home().ok_or(TempError::UnsafeRoot)?;
    let home = std::fs::canonicalize(home).map_err(|_error| TempError::UnsafeRoot)?;
    let local = std::env::var_os("LOCALAPPDATA").ok_or(TempError::UnsafeRoot)?;
    let local = std::fs::canonicalize(local).map_err(|_error| TempError::UnsafeRoot)?;
    if !local.starts_with(&home) {
        return Err(TempError::UnsafeRoot);
    }
    if !super::directory(&local.join("Temp")) {
        return Err(TempError::UnsafeRoot);
    }
    let expected = std::fs::canonicalize(local.join("Temp")).map_err(|_error| TempError::Scan)?;
    let actual = std::fs::canonicalize(std::env::temp_dir()).map_err(|_error| TempError::Scan)?;
    if !expected.starts_with(&local) || actual != expected {
        return Err(TempError::UnsafeRoot);
    }
    Ok(TempSource::Directory {
        path: actual,
        home: Some(home),
    })
}
#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "Shares the fallible Windows root-resolution interface"
)]
pub(super) fn source() -> Result<TempSource, TempError> {
    Ok(TempSource::Directory {
        path: std::env::temp_dir(),
        home: crate::config::user_home(),
    })
}
