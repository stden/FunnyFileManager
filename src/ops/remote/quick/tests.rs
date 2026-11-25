//! Purpose: Unit tests for FTP/FTPS quick-connect, YAML auto-save, and `DirectConnect` routing.

use super::*;
use crate::models::remote_work::{RemoteOutcome, RemoteTask};
use crate::models::{Panel, PanelKind, SessionName};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::{RemoteError, RemoteLink};
use crate::sessions::SessionStore;

fn seed_app(dir: &std::path::Path) -> (App, std::path::PathBuf) {
    let yml = dir.join("ffm.yml");
    std::fs::write(
        &yml,
        "network_drives:\n  - path: 'Z:\\'\n    remote: '\\\\example.test\\share'\n    protocol: smb\n",
    )
    .expect("seed yaml");
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.sessions = SessionStore::load_from(yml.clone());
    (app, yml)
}

#[test]
fn ftp_quick_connect_saves_yaml_preserving_network_drives_and_prompts_login() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, yml) = seed_app(temp.path());
    command(&mut app, None).expect("bare :ftp opens prompt");
    assert!(matches!(
        &app.overlay,
        Overlay::InputPrompt(p) if p.purpose() == &InputPurpose::FtpConnect(PanelSide::Left)
    ));
    app.close_overlay();

    command(&mut app, Some("ftps://alice@example.test:990/pub")).expect("save and prompt");
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("expected Remote login dialog");
    };
    assert!(matches!(&overlay.0, RemoteDialog::Login(_)));
    let saved = std::fs::read_to_string(&yml).expect("read ffm.yml");
    assert!(saved.contains("network_drives:"));
    assert!(saved.contains("example.test"));
    assert!(app
        .sessions
        .find(&SessionName::new("alice@example.test:990").expect("name"))
        .is_some());
}

fn queued_right_login(
    app: &mut App,
    yml: &std::path::Path,
) -> crate::models::remote_work::RemoteRequest {
    command(app, Some("ftps://alice@example.test:990/pub")).expect("save");
    let Overlay::Remote(overlay) = &app.overlay else {
        panic!("expected Login");
    };
    let RemoteDialog::Login(mut login) = overlay.0.clone() else {
        panic!("expected Login variant");
    };
    login.credentials.password = "transient-only".to_owned().into();
    app.close_overlay();
    assert!(!std::fs::read_to_string(yml)
        .expect("yml")
        .contains("transient-only"));
    login.side = Some(PanelSide::Right);
    crate::ops::remote::dialogs::login(app, login);
    app.remote_requests.take().expect("queued connect")
}

#[test]
fn ftp_quick_connect_preserves_side_on_retry() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, yml) = seed_app(temp.path());
    let request = queued_right_login(&mut app, &yml);
    crate::ops::remote::completion::apply(
        &mut app,
        request,
        Err(RemoteError::Failed("auth failed")),
    );
    let Overlay::Remote(retry) = &app.overlay else {
        panic!("retry dialog");
    };
    assert!(matches!(
        &retry.0,
        RemoteDialog::Login(l) if l.side == Some(PanelSide::Right)
    ));
}

#[test]
fn ftp_quick_connect_opens_remote_panel_on_target_side() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, yml) = seed_app(temp.path());
    let request = queued_right_login(&mut app, &yml);
    let link = RemoteLink::new(Box::new(FakeRemoteFs::default()));
    crate::ops::remote::completion::apply(
        &mut app,
        request,
        Ok(RemoteOutcome::Listed {
            link,
            path: crate::models::RemotePath::root(),
            entries: Vec::new(),
        }),
    );
    assert!(matches!(app.right_panel.kind, PanelKind::Remote(_)));
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn ftp_quick_connect_rejects_bad_urls_and_damaged_stores() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(command(&mut app, Some("http://example.test")).is_err());

    let yml = temp.path().join("ffm.yml");
    std::fs::write(&yml, "sessions: [broken\n").expect("damaged yaml");
    app.sessions = SessionStore::load_from(yml);
    submit(&mut app, PanelSide::Left, "ftp.example.test");
    assert!(matches!(app.status, Status::Error(_)));
    assert!(!app.overlay.is_active());
    assert!(matches!(
        RemoteTask::Connect {
            session: crate::values::session::parse_ftp_target("ftp.example.test").expect("s"),
            credentials: crate::remote::Credentials::default(),
        },
        RemoteTask::Connect { .. }
    ));
}

#[test]
fn prompt_ftp_targets_explicit_panel_side() {
    let temp = tempfile::TempDir::new().expect("temp");
    let (mut app, _yml) = seed_app(temp.path());
    prompt_ftp(&mut app, PanelSide::Right);
    assert!(matches!(
        &app.overlay,
        Overlay::InputPrompt(p) if p.purpose() == &InputPurpose::FtpConnect(PanelSide::Right)
    ));
}
