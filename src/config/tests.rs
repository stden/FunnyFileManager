//! Unit tests: defaults and the config path rule; the load/save behaviour is
//! in `tests/persistence.rs`.

use std::ffi::OsString;

use tempfile::TempDir;

use super::home::Platform;
use super::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

pub(super) mod persistence;
mod sharing;

#[test]
fn defaults_follow_far() {
    let settings = Settings::default();
    assert!(!settings.system.auto_save_setup);
    assert_eq!(
        usize::try_from(settings.system.history_size).unwrap(),
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
    variables: &'a [(&'static str, &'a Path)],
) -> impl Fn(&'static str) -> Option<OsString> + 'a {
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
    let appdata = Path::new("C:/Users/bob/AppData/Roaming");
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
    let relative = Path::new("relative/dir");
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

#[test]
fn the_lsp_key_is_the_name_serde_gives_the_field() {
    let value = serde_norway::to_value(Settings::default()).expect("settings serialize");
    assert!(value.get(LSP_KEY).is_some());
}

/// Opening a FIFO blocks until something writes to it, and the settings load
/// before the first frame: startup would hang.
#[cfg(unix)]
#[test]
fn load_does_not_block_on_a_fifo() {
    let temp = TempDir::new().expect("tempdir");
    let fifo = temp.path().join(document::FILE_NAME);
    crate::test_support::make_fifo(&fifo).expect("mkfifo");
    let loaded =
        crate::test_support::run_with_timeout(std::time::Duration::from_secs(5), move || {
            Settings::load_from(&fifo).map_err(|error| error.to_string())
        })
        .expect("loading settings must not block on a FIFO");
    assert_eq!(
        loaded,
        Err("settings.yaml is not a regular file".to_owned())
    );
}
