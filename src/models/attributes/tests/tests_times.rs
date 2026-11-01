//! Purpose: unit tests for the time fields: what they offer, what they ask
//! for, and how a bad time is refused.

use super::*;

/// A time field left as it opened asks for nothing, the rule every box
/// keeps: with Process subfolders on, the folder's own times would otherwise
/// be stamped on everything inside it.
#[test]
fn untouched_offered_times_ask_for_nothing() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, true)]);
    dialog.process_subfolders = true;

    let times = dialog.requested_times().expect("the offered text parses");

    assert_eq!(times.modified, None);
    assert_eq!(times.accessed, None);
}

/// Empty fields leave the time alone; garbage is refused by name.
#[test]
fn time_fields_parse_or_are_refused() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    dialog.modified.current.clear();
    dialog.accessed.current = "1999-12-31 23:59:59".to_owned();
    let times = dialog.requested_times().expect("valid");
    assert_eq!(times.modified, None);
    assert!(times.accessed.is_some());

    dialog.accessed.current = "yesterday".to_owned();
    let error = dialog.requested_times().expect_err("refused");
    assert!(error.to_string().contains("yesterday"), "{error}");
}

/// The refusal quotes the typed text and escapes what the status line could
/// not show as typed: a quote, a backslash, a tab. An apostrophe stays as it
/// is.
#[test]
fn a_refused_time_quotes_and_escapes_the_typed_text() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    dialog.modified.current.clear();
    dialog.accessed.current = "12:00\" C:\\2024\tit's".to_owned();

    let error = dialog.requested_times().expect_err("refused");

    assert_eq!(
        error.to_string(),
        r#"Access time: "12:00\" C:\\2024\tit's" is not a time (YYYY-MM-DD HH:MM:SS)"#
    );
}
