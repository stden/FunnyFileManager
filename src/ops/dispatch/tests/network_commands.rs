//! Purpose: Dispatcher tests for `:ftp` and `:netdrives` colon commands.

use super::*;

#[test]
fn ftp_and_netdrives_colon_commands_route_through_dispatcher() {
    let temp = TempDir::new().expect("temp");
    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    let mut app = make_app(&temp);

    execute_command(&mut app, "ftp").expect("bare ftp opens prompt");
    assert!(matches!(app.overlay, Overlay::InputPrompt(_)));
    app.close_overlay();

    execute_command(&mut app, "ftp ftp.example.test").expect("ftp connect");
    assert!(matches!(app.overlay, Overlay::Remote(_)));
    app.close_overlay();

    execute_command(&mut app, "netdrives connect").expect("netdrives connect");
    execute_command(&mut app, "netdrives edit").expect("netdrives edit");
    assert!(matches!(app.overlay, Overlay::Editor(_)));
    app.close_overlay();
    execute_command(&mut app, "netdrives list").expect("netdrives list");
}
