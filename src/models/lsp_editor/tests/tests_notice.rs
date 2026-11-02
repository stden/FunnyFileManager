//! Purpose: unit tests for the message row: the read-only reason and a
//! cleared notice.

use super::*;

#[test]
fn a_read_only_reason_reads_as_a_sentence() {
    assert_eq!(
        ReadOnlyReason::NoSettingsFile.to_string(),
        "no settings file for this session"
    );
    let reason = ReadOnlyReason::Document {
        path: "/srv/x/settings.yaml".into(),
        reason: "settings.yaml does not parse: expected a mapping of settings".to_owned(),
    };
    assert_eq!(
        reason.to_string(),
        "settings.yaml does not parse: expected a mapping of settings"
    );
}

#[test]
fn a_cleared_notice_leaves_the_message_row_to_the_changes() {
    let mut editor = editor(&json!({}));
    editor.set_notice(Notice::Info("Checked every server again.".to_owned()));
    editor.clear_notice();
    assert_eq!(editor.notice(), None);
}

/// The one sentence a read-only dialog gives, on its message row and for
/// every change it refuses.
#[test]
fn a_read_only_dialog_says_why_in_one_sentence() {
    let writable = editor(&json!({}));
    assert_eq!(writable.read_only_notice(), None);
    let read_only = LspEditor::new(
        Access::ReadOnly(ReadOnlyReason::NoSettingsFile),
        LspSettings::default(),
    );
    assert_eq!(
        read_only.read_only_notice(),
        Some(Notice::Error(
            "Read only: no settings file for this session.".to_owned()
        ))
    );
}
