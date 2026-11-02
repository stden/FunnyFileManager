//! Purpose: Tests for the text report the task list falls back to: the
//! command each platform runs.

use super::report_command;

#[cfg(not(windows))]
#[test]
fn unix_reports_with_ps_and_its_four_columns() {
    let command = report_command();
    assert_eq!(command.get_program(), "ps");
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, ["-eo", "pid,comm,%cpu,%mem"]);
}

#[cfg(windows)]
#[test]
fn windows_reports_with_tasklist() {
    let command = report_command();
    assert_eq!(command.get_program(), "tasklist");
    assert_eq!(command.get_args().count(), 0);
}
