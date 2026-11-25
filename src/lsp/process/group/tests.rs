//! Purpose: A failed reap is logged as a fixed line that never quotes the OS error.

use super::*;

#[test]
fn failed_reaping_reports_a_fixed_diagnostic_without_os_error_details() {
    let mut lines = Vec::new();
    report_reap(
        &Err(std::io::Error::other("sensitive process detail")),
        &mut |line| lines.push(line.to_owned()),
    );
    assert_eq!(lines, ["LSP process reap failed"]);
}
