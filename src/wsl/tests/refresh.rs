//! Purpose: Tests for the distribution list a refresh reads.

use super::*;

#[test]
fn refresh_reads_names_without_localized_verbose_columns() {
    let result = execute_with(&WslOperation::Refresh, |args, timeout| {
        assert_eq!(args, ["--list", "--quiet"]);
        assert_eq!(timeout, Duration::from_secs(20));
        Ok(success(utf16le(
            "Ubuntu-24.04\r\nDebian\r\nUbuntu-24.04\r\n",
        )))
    })
    .unwrap();
    assert_eq!(
        result,
        WslOutput::Distributions(vec![distribution("Ubuntu-24.04"), distribution("Debian")])
    );
}

#[test]
fn refresh_supports_empty_distribution_list() {
    assert_eq!(
        execute_with(&WslOperation::Refresh, |_, _| Ok(success("\r\n"))).unwrap(),
        WslOutput::Distributions(Vec::new())
    );
}

#[test]
fn refresh_rejects_invalid_names_and_incomplete_lists() {
    for name in ["../Ubuntu", "Ubuntu\\Root", "Ubuntu\tBad", "* Ubuntu"] {
        let error = execute_with(&WslOperation::Refresh, |_, _| Ok(success(name))).unwrap_err();
        assert!(error.contains("invalid distribution"), "{name}: {error}");
    }
    let error = execute_with(&WslOperation::Refresh, |_, _| {
        let mut captured = success("Ubuntu");
        captured.stdout.truncated = true;
        Ok(captured)
    })
    .unwrap_err();
    assert!(error.contains("output limit"));
}
