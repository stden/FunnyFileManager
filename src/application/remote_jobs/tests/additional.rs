//! Purpose: Direct remote saves preserve conflicts and explicit listings publish worker results.
use super::*;

#[test]
fn direct_save_detects_remote_conflicts_without_overwriting() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, link) = fixture(&temp);
    assert!(crate::ops::remote::actions::handle(&mut app, Action::Edit));
    finish(&mut RemoteJobs::default(), &mut app);
    let editor = app.get_editor_mut().expect("editor");
    editor.insert_char('X');
    crate::ops::remote::editor::save_direct(editor).expect("save");
    assert!(!editor.state.is_modified());
    link.lock()
        .upload(
            &path("/file.txt"),
            &mut b"external".as_slice(),
            WriteMode::Overwrite,
        )
        .expect("external change");
    editor.insert_char('Y');
    assert!(crate::ops::remote::editor::save_direct(editor).is_err());
    assert!(editor.state.is_modified());
    let mut bytes = Vec::new();
    link.lock()
        .download(&path("/file.txt"), &mut bytes)
        .expect("download");
    assert_eq!(bytes, b"external");
}

#[test]
fn explicit_directory_request_publishes_the_requested_snapshot() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let (mut app, link) = fixture(&temp);
    link.lock().mkdir(&path("/work")).expect("mkdir");
    crate::ops::remote::request_list(&mut app, path("/work"));
    finish(&mut RemoteJobs::default(), &mut app);
    let PanelKind::Remote(panel) = &app.left_panel.kind else {
        panic!("remote panel");
    };
    assert_eq!(panel.path(), &path("/work"));
    assert_eq!(panel.entries().len(), 1);
    assert_eq!(panel.entries()[0].name, "..");
}
