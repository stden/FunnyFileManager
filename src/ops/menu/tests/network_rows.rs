//! Purpose: Menu command tests for F9 network drive and FTP quick-connect rows.

use super::*;

#[test]
fn network_drives_and_ftp_menu_rows_open_expected_views() {
    let config = TempDir::new().expect("config");
    let _env = crate::test_support::EnvVarsGuard::set(&[(
        crate::test_support::CONFIG_HOME,
        config.path(),
    )]);
    let (_temp, mut app) = fixture();

    execute_result(&mut app, MenuCommand::ConnectNetworkDrives).expect("connect all");
    assert!(app.status_text().contains("No network drives"));

    execute_result(&mut app, MenuCommand::EditNetworkDrives).expect("edit ffm.yml");
    assert!(matches!(app.overlay, Overlay::Editor(_)));
    app.close_overlay();

    execute_result(&mut app, MenuCommand::FtpConnect).expect("ftp prompt");
    assert!(matches!(app.overlay, Overlay::InputPrompt(_)));
}
