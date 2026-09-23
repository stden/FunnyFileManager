//! Unit tests: defaults, the config path rule, and the load/save round trip.

use tempfile::TempDir;

use super::*;
use crate::test_support::EnvVarGuard;

/// The variable that names the config root on this platform.
const CONFIG_HOME: &str = if cfg!(windows) {
    "APPDATA"
} else {
    "XDG_CONFIG_HOME"
};

#[test]
fn defaults_follow_far() {
    let settings = Settings::default();
    assert!(!settings.system.auto_save_setup);
    assert_eq!(
        settings.system.history_size as usize,
        crate::history::HISTORY_CAP
    );
    assert!(settings.panel.show_hidden);
    assert!(settings.tree.auto_change_folder, "spec A2: default on");
    assert!(settings.interface.menu_bar, "the bar is always drawn today");
    assert!(settings.menu.wrap_cursor);
    assert_eq!(settings.command_line.prompt_format, "$p$g");
    assert_eq!(settings.descriptions.names, ["Descript.ion", "Files.bbs"]);
    assert_eq!(
        settings.folder_descriptions.names,
        ["Dirinfo", "Folder.txt"]
    );
    assert!(!settings.viewer.wrap);
    assert_eq!(settings.colors.theme, Theme::FarBlue);
    assert_eq!(settings.languages.main, Lang::En);
    assert!(settings.shortcuts.iter().all(Option::is_none));
    assert_eq!(settings.panel_mode(PanelView::Full).columns, "N,S,D,T");
    assert!(settings.panel_mode(PanelView::Detailed).fullscreen);
    assert_eq!(PanelView::Alternative.index(), 9);
}

#[test]
fn the_path_sits_under_the_config_home() {
    let temp = TempDir::new().expect("tempdir");
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    assert_eq!(
        Settings::path(),
        Some(temp.path().join("ffm").join("settings.json"))
    );
}

#[cfg(not(windows))]
#[test]
fn a_relative_xdg_config_home_is_ignored() {
    let _home = EnvVarGuard::set(CONFIG_HOME, std::path::Path::new("relative/dir"));
    let path = Settings::path();
    assert!(
        path.as_ref().is_none_or(|path| path.is_absolute()),
        "{path:?} must not be launch-directory relative"
    );
}

#[test]
fn load_survives_a_missing_and_a_corrupt_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    assert_eq!(Settings::load(), Settings::default());
    fs::create_dir_all(temp.path().join("ffm"))?;
    fs::write(temp.path().join("ffm/settings.json"), "{ not json")?;
    assert_eq!(Settings::load(), Settings::default());
    fs::write(
        temp.path().join("ffm/settings.json"),
        r#"{"system":{"wipe_on_delete":true}}"#,
    )?;
    let partial = Settings::load();
    assert!(
        partial.system.wipe_on_delete,
        "a partial file loads its fields"
    );
    assert!(partial.panel.show_hidden, "and defaults the rest");
    Ok(())
}

#[test]
fn save_then_load_round_trips_every_page() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let mut settings = Settings::default();
    settings.system.auto_save_setup = true;
    settings.shortcuts[3] = Some(PathBuf::from("/srv"));
    settings
        .mask_groups
        .groups
        .push(("code".to_owned(), "*.rs,*.py".to_owned()));
    settings.colors.theme = Theme::Grey;
    settings.panel_modes.modes[0].columns = "N,S".to_owned();
    settings.save()?;
    assert!(temp.path().join("ffm/settings.json").is_file());
    assert_eq!(Settings::load(), settings);
    Ok(())
}

#[test]
fn load_survives_an_unreadable_settings_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    fs::create_dir_all(temp.path().join("ffm").join("settings.json"))?;
    assert_eq!(Settings::load(), Settings::default());
    Ok(())
}
