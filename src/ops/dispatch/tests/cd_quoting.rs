//! Purpose: `cd` into folders with spaces or quotes, and `~` with and without a home.

use super::*;

// ─── cd: spaces, quotes and `~` ───────────────────────────────────────

#[test]
fn test_cd_into_directory_with_spaces() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

    let mut app = make_app(&temp);
    execute_command(&mut app, "cd My Documents").expect("cd");
    assert_eq!(
        app.get_active_panel().path.file_name().unwrap(),
        "My Documents"
    );
}

#[test]
fn test_cd_into_quoted_directory_with_spaces() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

    let mut app = make_app(&temp);
    execute_command(&mut app, "cd \"My Documents\"").expect("cd");
    assert_eq!(
        app.get_active_panel().path.file_name().unwrap(),
        "My Documents"
    );
}

/// The completed line the tab-completer produces must execute unchanged.
#[test]
fn test_completed_cd_line_executes() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir(temp.path().join("My Documents")).expect("mkdir");

    let mut app = make_app(&temp);
    let completion = crate::complete::complete("cd My", temp.path()).expect("completion");
    execute_command(&mut app, &completion.line).expect("cd");
    assert_eq!(
        app.get_active_panel().path.file_name().unwrap(),
        "My Documents"
    );
}

/// `cd ~` used to fail: `~/x` is a relative path, so it was joined onto the
/// panel directory. Asserted structurally — other tests swap $HOME
/// concurrently, so the exact destination is not stable, only that it is a
/// real directory outside the panel we started in.
/// Put `previous` back into the environment, or clear the variable when
/// there was nothing to restore.
fn restore_env(name: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

#[test]
fn restore_env_sets_and_clears() {
    let _lock = crate::test_support::env_lock();
    let previous = std::env::var_os("FFM_DISPATCH_RESTORE_PROBE");

    restore_env("FFM_DISPATCH_RESTORE_PROBE", Some("probe".into()));
    assert_eq!(
        std::env::var_os("FFM_DISPATCH_RESTORE_PROBE").as_deref(),
        Some("probe".as_ref())
    );
    restore_env("FFM_DISPATCH_RESTORE_PROBE", None);
    assert!(std::env::var_os("FFM_DISPATCH_RESTORE_PROBE").is_none());

    restore_env("FFM_DISPATCH_RESTORE_PROBE", previous);
}

#[test]
fn test_cd_tilde_opens_home_not_a_subdirectory_of_cwd() {
    let _lock = crate::test_support::env_lock();
    // Point HOME at a temp directory instead of skipping when it is unset,
    // so the test is deterministic in every environment.
    let prev_home = std::env::var_os("HOME");
    let home = TempDir::new().expect("home tempdir");
    std::env::set_var("HOME", home.path());

    let temp = TempDir::new().expect("tempdir");
    let mut app = make_app(&temp);

    let result = execute_command(&mut app, "cd ~");
    let landed = app.get_active_panel().path.clone();
    restore_env("HOME", prev_home);

    result.expect("cd ~");
    assert!(
        !landed.starts_with(temp.path()),
        "~ must not be resolved relative to the panel directory"
    );
}

/// With neither `$HOME` nor `%USERPROFILE%`, `~` is left unexpanded. Joining
/// it onto the panel as a relative name would open `./~` if that existed;
/// the command must fail (or stay put) rather than invent a home directory.
#[test]
fn test_cd_tilde_without_home_does_not_invent_a_directory() {
    let temp = TempDir::new().expect("tempdir");
    // A real `~` subdirectory would make a wrong expansion look successful.
    fs::create_dir(temp.path().join("~")).expect("mkdir literal tilde");

    let _lock = crate::test_support::env_lock();
    let prev_home = std::env::var_os("HOME");
    let prev_profile = std::env::var_os("USERPROFILE");
    std::env::remove_var("HOME");
    std::env::remove_var("USERPROFILE");

    let mut app = make_app(&temp);
    let before = app.get_active_panel().path.clone();
    let result = execute_command(&mut app, "cd ~");

    // Restore before any assert so a failure cannot leak empty HOME.
    restore_env("HOME", prev_home);
    restore_env("USERPROFILE", prev_profile);

    assert!(
        result.is_err() || app.get_active_panel().path == before,
        "without a home variable, cd ~ must not land on a panel-local '~' dir"
    );
    assert_ne!(
        app.get_active_panel()
            .path
            .file_name()
            .and_then(|n| n.to_str()),
        Some("~"),
        "panel must not open the literal '~' subdirectory as home"
    );
}
