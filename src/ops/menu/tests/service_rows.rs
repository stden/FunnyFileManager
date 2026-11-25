//! Purpose: Menu command tests for F9 Jira, MCP services, and AI assistant rows.

use super::*;

#[test]
fn service_menu_rows_open_expected_overlays() {
    let config = TempDir::new().expect("config");
    let _env = crate::test_support::EnvVarsGuard::set(&[(
        crate::test_support::CONFIG_HOME,
        config.path(),
    )]);
    let (_temp, mut app) = fixture();

    execute_result(&mut app, MenuCommand::JiraIssues).expect("jira");
    assert!(matches!(app.overlay, Overlay::Jira(_)));
    app.close_overlay();

    execute_result(&mut app, MenuCommand::McpServices).expect("mcps");
    assert!(matches!(app.overlay, Overlay::McpServices(_)));
    app.close_overlay();

    execute_result(&mut app, MenuCommand::AiAssistant).expect("ai");
    assert!(matches!(app.overlay, Overlay::Ai(_)));
    app.close_overlay();

    execute_result(&mut app, MenuCommand::ThingsCatalog).expect("things");
    assert!(matches!(app.overlay, Overlay::Things(_)));
    app.close_overlay();

    execute_result(&mut app, MenuCommand::CloudflareDns).expect("cloudflare");
    assert!(matches!(app.overlay, Overlay::CloudflareDns(_)));
}

#[test]
fn spooler_and_volume_maintenance_menu_rows_dispatch_per_platform() {
    let (_temp, mut app) = fixture();

    let spooler = execute_result(&mut app, MenuCommand::StartSpooler);
    let volume = execute_result(&mut app, MenuCommand::VolumeMaintenance);
    #[cfg(not(windows))]
    {
        assert_eq!(
            spooler.expect_err("non-windows").kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            volume.expect_err("non-windows").kind(),
            io::ErrorKind::Unsupported
        );
    }
    #[cfg(windows)]
    {
        drop(spooler);
        drop(volume);
    }
}
