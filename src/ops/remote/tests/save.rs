//! A saved remote revision is applied only to its owning editor.

use super::super::{completion, editor};
use crate::models::remote_work::{
    FileLocation, RemoteOutcome, RemoteRequest, RemoteTarget, RemoteTask,
};
use crate::models::{AfterSave, App, EditState, Fingerprint, Overlay, PanelKind, PanelSide};
use crate::remote::WriteMode;

fn opened() -> (tempfile::TempDir, App, RemoteTarget, FileLocation) {
    let dir = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(dir.path(), &[("/file", 3)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel")
    };
    let path = panel.path().join("file").expect("path");
    panel
        .link()
        .lock()
        .upload(&path, &mut b"old".as_slice(), WriteMode::Overwrite)
        .expect("seed");
    let target = RemoteTarget::Panel {
        side: PanelSide::Left,
        link: panel.link().clone(),
        path: panel.path().clone(),
    };
    let source = FileLocation::Remote {
        link: panel.link().clone(),
        path,
    };
    editor::open(&mut app, source.clone(), b"old", true).expect("editor");
    (dir, app, target, source)
}

fn request(
    target: RemoteTarget,
    source: FileLocation,
    revision: u64,
    then: AfterSave,
) -> RemoteRequest {
    RemoteRequest {
        id: 1,
        target,
        task: RemoteTask::Save {
            source,
            bytes: b"saved".to_vec(),
            expected: Fingerprint::of(b"old"),
            overwrite: false,
            then,
            revision,
        },
    }
}

#[test]
fn successful_save_keeps_the_editor_open_and_refreshes_its_fingerprint() {
    let (_dir, mut app, target, source) = opened();
    let revision = app.get_editor().expect("editor").lsp.revision.0;
    completion::apply(
        &mut app,
        request(target, source, revision, AfterSave::Stay),
        Ok(RemoteOutcome::Saved),
    );
    let editor = app.get_editor().expect("editor stays open");
    assert_eq!(editor.state, EditState::Clean);
    assert_eq!(
        editor.remote.as_ref().expect("remote").fingerprint,
        Fingerprint::of(b"saved")
    );
    assert_eq!(editor.notice.as_deref(), Some("Saved to remote server"));
    assert_eq!(std::fs::read(&editor.path).expect("working copy"), b"saved");
}

#[test]
fn save_then_close_does_not_close_if_the_user_made_later_edits() {
    let (_dir, mut app, target, source) = opened();
    let revision = app.get_editor().expect("editor").lsp.revision.0;
    app.get_editor_mut().expect("editor").insert_char('!');
    completion::apply(
        &mut app,
        request(target, source, revision, AfterSave::Close),
        Ok(RemoteOutcome::Saved),
    );
    let editor = app.get_editor().expect("newer edit stays open");
    assert_eq!(editor.state, EditState::Modified);
    assert_eq!(editor.content(), "!old");
    assert_eq!(
        editor.remote.as_ref().expect("remote").fingerprint,
        Fingerprint::of(b"saved")
    );
}

#[test]
fn save_then_close_closes_an_unchanged_editor() {
    let (_dir, mut app, target, source) = opened();
    let revision = app.get_editor().expect("editor").lsp.revision.0;
    completion::apply(
        &mut app,
        request(target, source, revision, AfterSave::Close),
        Ok(RemoteOutcome::Saved),
    );
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn direct_editor_save_writes_the_real_fake_host_and_clears_modified_state() {
    let (_dir, mut app, _target, source) = opened();
    let editor = app.get_editor_mut().expect("editor");
    editor.insert_char('!');
    editor.save().expect("remote save");
    assert_eq!(editor.state, EditState::Clean);
    let FileLocation::Remote { link, path } = source else {
        panic!("remote")
    };
    let mut bytes = Vec::new();
    link.lock()
        .download(&path, &mut bytes)
        .expect("saved content");
    assert_eq!(bytes, b"!old");
}
