//! Purpose: FTP command validation and sanitized suppaftp error mapping.

use crate::remote::RemoteError;
use crate::values::remote_path::RemotePath;
use suppaftp::FtpError;

/// RFC 959 reply codes indicating the server does not implement an optional command.
const UNSUPPORTED_CODES: [u32; 5] = [202, 500, 501, 502, 504];

pub(super) fn argument(path: &RemotePath) -> Result<&str, RemoteError> {
    if path.as_str().chars().any(char::is_control) {
        return Err(RemoteError::InvalidInput(
            "FTP cannot address names containing control characters",
        ));
    }
    Ok(path.as_str())
}

pub(super) fn unsupported(error: &FtpError) -> bool {
    matches!(
        error,
        FtpError::UnexpectedResponse(reply) if UNSUPPORTED_CODES.contains(&reply.status.code())
    )
}

pub(super) fn failed(error: FtpError) -> RemoteError {
    match error {
        FtpError::ConnectionError(error) => error.into(),
        _ => RemoteError::Failed("FTP server refused the operation"),
    }
}
