//! Purpose: System-profile background sessions load and save inside a private home.

use super::*;

const ROOT_ENV: &str = "FFM_BACKGROUND_SYSTEM_TEST_ROOT";
const TEST: &str =
    "bootstrap::background::tests_system::system_profile_persists_only_in_its_private_home";

#[test]
fn system_profile_persists_only_in_its_private_home() -> io::Result<()> {
    if let Some(root) = std::env::var_os(ROOT_ENV) {
        return exercise(&std::path::PathBuf::from(root));
    }
    let root = tempfile::TempDir::new()?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", TEST, "--nocapture"])
        .env(ROOT_ENV, root.path())
        .env("HOME", root.path().join("home"))
        .env("USERPROFILE", root.path().join("home"))
        .env("APPDATA", root.path().join("config"))
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env_remove("FFM_LOG")
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn exercise(root: &Path) -> io::Result<()> {
    let work = root.join("work");
    std::fs::create_dir_all(&work)?;
    let mut session = app(&work, Profile::System)?;
    assert_eq!(session.live.terminal_mode, TerminalMode::Headless);
    assert!(session.live.elevation);
    assert!(session.clipboard.backend_mut().is_some());
    session.config.system.auto_save_setup = true;
    session.clipboard.arm(std::time::Instant::now());
    let settings = crate::config::Settings::path().expect("private settings path");
    assert!(settings.starts_with(root));
    finish(&mut session, Profile::System);
    assert!(
        settings.is_file(),
        "system finish saves setup in its private home"
    );
    let restored = crate::config::Settings::load();
    assert!(restored.system.auto_save_setup);
    Ok(())
}
