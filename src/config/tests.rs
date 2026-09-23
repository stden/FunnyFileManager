//! Unit tests: defaults and the config path rule; the load/save behaviour is
//! in `tests/persistence.rs`.

use tempfile::TempDir;

use super::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

mod persistence;
mod sharing;

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
        Some(temp.path().join("ffm").join("settings.yaml"))
    );
}

/// An environment holding exactly `variables`, for the pure path rules.
fn environment<'a>(
    variables: &'a [(&'static str, &'a std::path::Path)],
) -> impl Fn(&'static str) -> Option<std::ffi::OsString> + 'a {
    |name| {
        variables
            .iter()
            .find(|(variable, _)| *variable == name)
            .map(|(_, value)| value.as_os_str().to_owned())
    }
}

#[test]
fn windows_keeps_the_config_directory_under_appdata() {
    let temp = TempDir::new().expect("tempdir");
    let appdata = std::path::Path::new("C:/Users/bob/AppData/Roaming");
    let unix_homes = [("XDG_CONFIG_HOME", temp.path()), ("HOME", temp.path())];
    assert_eq!(
        Platform::Windows.dir(environment(&[
            ("APPDATA", appdata),
            unix_homes[0],
            unix_homes[1]
        ])),
        Some(appdata.join("ffm"))
    );
    assert_eq!(
        Platform::Windows.dir(environment(&unix_homes)),
        None,
        "Windows never falls back to the Unix variables"
    );
}

#[test]
fn unix_prefers_an_absolute_xdg_config_home_to_home() {
    let xdg = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("tempdir");
    let variables = [
        ("XDG_CONFIG_HOME", xdg.path()),
        ("HOME", home.path()),
        ("APPDATA", home.path()),
    ];
    assert_eq!(
        Platform::Unix.dir(environment(&variables)),
        Some(xdg.path().join("ffm"))
    );
    assert_eq!(
        Platform::Unix.dir(environment(&[("HOME", home.path())])),
        Some(home.path().join(".config").join("ffm"))
    );
}

#[test]
fn unix_ignores_a_relative_xdg_config_home() {
    let home = TempDir::new().expect("tempdir");
    let relative = std::path::Path::new("relative/dir");
    assert_eq!(
        Platform::Unix.dir(environment(&[
            ("XDG_CONFIG_HOME", relative),
            ("HOME", home.path())
        ])),
        Some(home.path().join(".config").join("ffm")),
        "settings must never land in the launch directory"
    );
    assert_eq!(
        Platform::Unix.dir(environment(&[("XDG_CONFIG_HOME", relative)])),
        None
    );
}

#[test]
fn the_current_platform_rule_reads_the_process_environment() {
    let temp = TempDir::new().expect("tempdir");
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    assert_eq!(dir(), Some(temp.path().join("ffm")));
    assert_eq!(
        Platform::CURRENT,
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        }
    );
}
