//! Unit tests for `decode_system_times` zero-status Win32 error fallback.

use super::*;

#[test]
fn decode_system_times_returns_error_when_win32_reports_zero() {
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    assert!(decode_system_times(0, zero, zero, zero).is_err());
}
