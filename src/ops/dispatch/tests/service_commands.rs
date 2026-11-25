//! Purpose: Dispatcher tests for `:jira`, `:mcps`, `:ai`, and `:ask` commands.

use super::*;

#[test]
fn service_colon_commands_route_to_their_overlays() {
    let temp = TempDir::new().expect("temp");
    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    let mut app = make_app(&temp);

    execute_command(&mut app, "jira").expect("jira");
    assert!(matches!(app.overlay, Overlay::Jira(_)));
    app.close_overlay();

    execute_command(&mut app, "mcps").expect("mcps");
    assert!(matches!(app.overlay, Overlay::McpServices(_)));
    app.close_overlay();

    execute_command(&mut app, "ai codex").expect("ai");
    assert!(matches!(app.overlay, Overlay::Ai(_)));
    app.close_overlay();

    execute_command(&mut app, "ask list files").expect("ask");
    assert!(matches!(app.overlay, Overlay::Ai(_)));
    app.close_overlay();

    execute_command(&mut app, "lsp").expect("lsp");
    assert!(matches!(app.overlay, Overlay::LspEditor(_)));
    app.close_overlay();
}

#[test]
fn start_spooler_routes_and_reports_injected_outcome() {
    let temp = TempDir::new().expect("temp");
    let mut app = make_app(&temp);

    let outcome = execute_command(&mut app, "start-spooler");
    #[cfg(not(windows))]
    assert_eq!(
        outcome.expect_err("non-windows").kind(),
        io::ErrorKind::Unsupported
    );
    #[cfg(windows)]
    drop(outcome);

    cmd_start_spooler_with(&mut app, || Ok("Print Spooler start requested")).expect("ok");
    assert_eq!(
        app.status,
        Status::CommandOutput("Print Spooler start requested".into())
    );
    assert!(cmd_start_spooler_with(&mut app, || Err(io::Error::other("denied"))).is_err());
}
