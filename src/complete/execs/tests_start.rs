//! Purpose: Starting the PATH scan can fail without blocking command completion.

use super::report_spawn_result_to;

#[test]
fn a_rejected_path_scan_reports_its_failure() {
    let rejected: std::io::Result<()> = Err(std::io::Error::other("thread quota reached"));
    let mut message = String::new();
    report_spawn_result_to(rejected, |line| {
        message = line.to_owned();
    });
    assert_eq!(message, "PATH scan: thread quota reached");
}
