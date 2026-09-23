//! Unit tests: `Shift+F9` writes the settings file and names it.

use tempfile::TempDir;

use super::run;
use crate::config::Settings;
use crate::models::App;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

#[test]
fn save_setup_writes_the_settings_file_and_reports_its_path() {
    let temp = TempDir::new().expect("test setup");
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let mut app = App::new().expect("test setup");
    app.config.system.wipe_on_delete = true;

    run(&mut app);

    let path = Settings::path().expect("a config root is set");
    assert!(path.is_file(), "{} was written", path.display());
    assert!(
        Settings::load().system.wipe_on_delete,
        "and holds the change"
    );
    assert!(app.status.to_string().contains(&path.display().to_string()));
}

#[test]
fn save_setup_reports_when_the_config_directory_cannot_be_created() {
    let temp = TempDir::new().expect("test setup");
    let blocked = temp.path().join("blocked");
    std::fs::write(&blocked, "not a directory").expect("file");
    let _home = EnvVarGuard::set(CONFIG_HOME, &blocked);
    let mut app = App::new().expect("test setup");
    run(&mut app);
    assert!(
        app.status.to_string().contains("Save setup"),
        "{}",
        app.status
    );
}

#[test]
fn save_setup_reports_a_settings_file_it_will_not_overwrite() {
    let temp = TempDir::new().expect("test setup");
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let path = Settings::path().expect("a config root is set");
    std::fs::create_dir_all(path.parent().expect("config dir")).expect("config dir");
    std::fs::write(&path, "system: [\n").expect("a broken file");
    let mut app = App::new().expect("test setup");

    run(&mut app);

    let status = app.status.to_string();
    assert!(
        status.contains("Save setup: Not saved: settings.yaml does not parse"),
        "{status}"
    );
    assert_eq!(std::fs::read_to_string(&path).expect("kept"), "system: [\n");
}
