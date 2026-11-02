//! Purpose: Remote panel operations run through worker completion and protect hidden local files.

use super::*;
use crate::action::Action;
use crate::models::{Overlay, Panel, PanelKind, Protocol, RemotePath, RowMotion};
use crate::remote::{fake::FakeRemoteFs, RemoteLink, WriteMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("remote path")
}

fn fixture(temp: &tempfile::TempDir) -> (App, RemoteLink) {
    let mut host = FakeRemoteFs::with_files(&[("/file.txt", 3)]);
    crate::remote::RemoteFs::upload(
        &mut host,
        &path("/file.txt"),
        &mut b"abc".as_slice(),
        WriteMode::Overwrite,
    )
    .expect("remote content");
    let mut app = App::with_panels(
        Panel::from_entries(temp.path().to_path_buf(), Vec::new()),
        Panel::new(temp.path().to_path_buf()).expect("local panel"),
    );
    let session = crate::models::Session::new(
        Protocol::Sftp,
        crate::models::HostName::new("example.test").expect("host"),
        crate::models::Port::SSH,
        None,
    );
    crate::ops::remote::open_with(&mut app, &session, Box::new(host)).expect("open remote");
    let PanelKind::Remote(remote) = &mut app.left_panel.kind else {
        panic!("remote panel");
    };
    remote.move_cursor(RowMotion::Next);
    let link = remote.link().clone();
    (app, link)
}

fn key(app: &mut App, code: KeyCode) {
    assert!(crate::popup::process_popup_input(
        app,
        KeyEvent::new(code, KeyModifiers::NONE)
    ));
}

fn finish(jobs: &mut RemoteJobs, app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !app.remote_requests.is_idle() {
        jobs.pump(app);
        assert!(
            std::time::Instant::now() < deadline,
            "remote worker finishes"
        );
        std::thread::yield_now();
    }
}

#[test]
fn remote_edit_saves_to_host_and_delete_cannot_touch_hidden_local_files() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    std::fs::write(temp.path().join("file.txt"), b"local").expect("hidden local file");
    let (mut app, link) = fixture(&temp);
    let mut jobs = RemoteJobs::default();
    assert!(crate::ops::remote::actions::handle(&mut app, Action::Edit));
    finish(&mut jobs, &mut app);
    app.get_editor_mut().expect("editor").insert_char('X');
    key(&mut app, KeyCode::F(2));
    finish(&mut jobs, &mut app);
    assert!(!app.get_editor().expect("editor").state.is_modified());
    let mut bytes = Vec::new();
    link.lock()
        .download(&path("/file.txt"), &mut bytes)
        .expect("saved remote");
    assert_eq!(bytes, b"Xabc");
    key(&mut app, KeyCode::Esc);
    assert!(crate::ops::remote::actions::handle(
        &mut app,
        Action::DeleteConfirm
    ));
    key(&mut app, KeyCode::Enter);
    finish(&mut jobs, &mut app);
    assert!(link.lock().stat(&path("/file.txt")).is_err());
    assert_eq!(
        std::fs::read(temp.path().join("file.txt")).expect("local survives"),
        b"local"
    );
}

#[test]
fn local_to_remote_copy_uses_the_visible_remote_destination() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    std::fs::write(temp.path().join("upload.txt"), b"uploaded").expect("local file");
    let (mut app, link) = fixture(&temp);
    app.active_panel = crate::models::PanelSide::Right;
    assert!(app.right_panel.select_named("upload.txt"));
    assert!(crate::ops::remote::actions::handle(&mut app, Action::Copy));
    key(&mut app, KeyCode::Enter);
    finish(&mut RemoteJobs::default(), &mut app);
    let mut bytes = Vec::new();
    link.lock()
        .download(&path("/upload.txt"), &mut bytes)
        .expect("uploaded file");
    assert_eq!(bytes, b"uploaded");
    assert_eq!(
        std::fs::read(temp.path().join("upload.txt")).expect("source kept"),
        b"uploaded"
    );
}

#[test]
fn late_download_never_reopens_a_panel_or_editor_after_leaving() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, _) = fixture(&temp);
    assert!(crate::ops::remote::actions::handle(&mut app, Action::Edit));
    app.left_panel.kind = PanelKind::Files;
    finish(&mut RemoteJobs::default(), &mut app);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(matches!(app.left_panel.kind, PanelKind::Files));
}

mod additional;
mod mailbox;
