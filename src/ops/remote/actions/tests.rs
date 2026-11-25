use super::{permissions, selection, SelectionCommand};
use crate::test_support::netbox::app_in;

#[test]
fn remote_only_permission_and_selection_helpers_leave_local_files_untouched() {
    let directory = tempfile::TempDir::new().expect("temporary directory");
    let file = directory.path().join("file");
    std::fs::write(&file, b"local").expect("local file");
    let mut app = app_in(directory.path());
    assert!(app.left_panel.select_named("file"));
    permissions(&mut app).expect("local panel ignored");
    selection(&mut app, SelectionCommand::SelectGroup);
    assert!(!app.overlay.is_active());
    assert!(app.remote_requests.is_idle());
    assert_eq!(std::fs::read(file).expect("local file remains"), b"local");
}

#[test]
fn remote_mark_commands_preserve_selection_history() {
    let directory = tempfile::TempDir::new().expect("temporary directory");
    let mut app =
        crate::test_support::netbox::app_on_host(directory.path(), &[("/a", 1), ("/b", 1)], None);
    if let crate::models::PanelKind::Remote(panel) = &mut app.left_panel.kind {
        panel.move_cursor(crate::models::RowMotion::Next);
    }
    for (command, expected) in [
        (SelectionCommand::ToggleMark, "a"),
        (SelectionCommand::InvertSelection, "b"),
        (SelectionCommand::RestoreSelection, "a"),
    ] {
        selection(&mut app, command);
        let crate::models::PanelKind::Remote(panel) = &app.left_panel.kind else {
            panic!("remote")
        };
        let names: Vec<_> = ["a", "b"]
            .into_iter()
            .filter(|name| panel.marked(name))
            .collect();
        assert_eq!(names, [expected]);
    }
}

#[test]
fn remote_parent_has_no_permissions_target_and_refresh_queues_its_listing() {
    let directory = tempfile::TempDir::new().expect("temporary directory");
    let mut app = crate::test_support::netbox::app_on_host(directory.path(), &[], None);
    assert!(super::handle(
        &mut app,
        crate::action::Action::File(crate::action::FileCommand::Attributes)
    ));
    assert!(!app.overlay.is_active());
    assert!(super::handle(&mut app, crate::action::Action::Refresh));
    assert!(matches!(
        app.remote_requests.take().expect("refresh").task,
        crate::models::remote_work::RemoteTask::Refresh(_)
    ));
}
