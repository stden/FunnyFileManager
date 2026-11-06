//! Purpose: a confirmed overwrite of a remote document queues its upload with
//! the overwrite flag set, and leaves the local scratch copy as it was.

use super::*;
use crate::models::remote_work::{FileLocation, RemoteTask};
use crate::models::{
    AfterSave, Editor, Fingerprint, Overlay, OverwritePrompt, PanelKind, RemoteDocument,
};
use crate::test_support::netbox::app_on_host;

/// An editor on `/remote.txt` of a fake host whose buffer says `new`, while
/// the scratch file and the remembered fingerprint still hold `old`.
fn remote_editor() -> (TempDir, App, std::path::PathBuf) {
    let temp = TempDir::new().expect("temporary directory");
    let mut app = app_on_host(temp.path(), &[("/remote.txt", 3)], None);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel");
    };
    let source = FileLocation::Remote {
        link: panel.link().clone(),
        path: panel.path().join("remote.txt").expect("remote file"),
    };
    let scratch = temp.path().join("remote.txt");
    fs::write(&scratch, b"old").expect("scratch file");
    let mut document = Editor::from_content("remote".to_owned(), scratch.clone(), "new");
    document.remote = Some(Box::new(RemoteDocument {
        location: source,
        fingerprint: Fingerprint::of(b"old"),
        temporary: std::sync::Arc::new(TempDir::new().expect("scratch owner")),
    }));
    app.set_overlay(Overlay::Editor(document));
    (temp, app, scratch)
}

#[test]
fn confirmed_remote_overwrite_queues_a_save_without_writing_the_scratch_file() {
    let (_temp, mut app, scratch) = remote_editor();
    save::confirm_overwrite(
        &mut app,
        KeyCode::Char('y'),
        OverwritePrompt::new(AfterSave::Stay),
    );
    assert!(matches!(app.remote_requests.take().expect("save").task,
        RemoteTask::Save { overwrite: true, bytes, .. } if bytes == b"new"));
    assert_eq!(fs::read(scratch).expect("scratch remains"), b"old");
    assert!(app.get_editor().is_some());
}
