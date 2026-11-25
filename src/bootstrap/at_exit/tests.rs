//! Purpose: Exit reporting leaves every failure on stderr, in order, once the
//! terminal is the user's again.

use super::*;

#[test]
fn exit_reports_statistics_recording_and_session_errors_together() {
    let mut lines = Vec::new();
    let failed_run = Err(io::Error::new(io::ErrorKind::BrokenPipe, "session ended"));
    report_problems(
        Err(io::Error::other("statistics blocked")),
        Some("recorder full".into()),
        &failed_run,
        |line| lines.push(line.to_owned()),
    );
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("could not save action statistics: statistics blocked"));
    assert!(lines[1].contains("recording stopped: recorder full"));
    assert!(lines[2].contains("BrokenPipe"));
    assert!(lines[2].contains("session ended"));
}
