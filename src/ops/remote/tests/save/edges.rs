use super::{completion, opened, request};
use crate::models::remote_work::{FileLocation, RemoteOutcome};
use crate::models::{AfterSave, Fingerprint, RemotePath};

#[test]
fn a_remote_save_reports_a_failed_working_copy_refresh() {
    let (_dir, mut app, target, source) = opened();
    let editor = app.get_editor_mut().expect("editor");
    let revision = editor.lsp.revision.0;
    std::fs::remove_file(&editor.path).expect("remove old scratch file");
    std::fs::create_dir(&editor.path).expect("scratch path occupied by directory");
    completion::apply(
        &mut app,
        request(target, source, revision, AfterSave::Stay),
        Ok(RemoteOutcome::Saved),
    );
    let editor = app.get_editor().expect("editor");
    assert!(editor
        .notice
        .as_deref()
        .expect("notice")
        .contains("temporary working copy could not be refreshed"));
    assert_eq!(
        editor.remote.as_ref().expect("remote").fingerprint,
        Fingerprint::of(b"saved")
    );
}

#[test]
fn a_late_save_for_a_different_document_does_not_mark_the_editor_saved() {
    let (_dir, mut app, target, mut source) = opened();
    let FileLocation::Remote { path, .. } = &mut source else {
        panic!("remote source")
    };
    *path = RemotePath::parse("/other").expect("other path");
    completion::apply(
        &mut app,
        request(target, source, 0, AfterSave::Close),
        Ok(RemoteOutcome::Saved),
    );
    let editor = app.get_editor().expect("unrelated editor remains");
    assert_eq!(
        editor.remote.as_ref().expect("remote").fingerprint,
        Fingerprint::of(b"old")
    );
    assert_ne!(editor.notice.as_deref(), Some("Saved to remote server"));
}

#[test]
fn closing_the_remote_panel_reports_a_save_error_without_losing_the_buffer() {
    let (_dir, mut app, _target, _source) = opened();
    app.left_panel.kind = crate::models::PanelKind::Files;
    assert!(super::editor::request_save(
        &mut app,
        AfterSave::Stay,
        false
    ));
    assert!(app.remote_requests.is_idle());
    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(editor.content(), "old");
    assert_eq!(editor.notice.as_deref(), Some("no remote panel is open"));
}

#[test]
fn an_unrepresentable_character_prevents_queuing_or_writing_a_remote_save() {
    let (_dir, mut app, _target, source) = opened();
    let editor = app.get_editor_mut().expect("editor");
    editor.insert_char('🦀');
    editor.format.encoding = crate::encoding::TextEncoding::Windows1252;
    assert!(editor.save().is_err());
    assert!(super::editor::request_save(
        &mut app,
        AfterSave::Stay,
        false
    ));
    assert!(app.remote_requests.is_idle());
    assert!(app
        .get_editor()
        .expect("buffer remains")
        .content()
        .contains('🦀'));
    let FileLocation::Remote { link, path } = source else {
        panic!("remote source")
    };
    let mut bytes = Vec::new();
    link.lock()
        .download(&path, &mut bytes)
        .expect("original contents");
    assert_eq!(bytes, b"old");
}
