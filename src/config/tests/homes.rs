//! Unit tests: the one rule for where the user's files live. The home and
//! each XDG base directory count only as absolute paths: an empty `HOME=`
//! put the settings, the histories and a `.profile` in the launch directory.

use std::path::Path;

use tempfile::TempDir;

use super::super::home::Platform;
use super::environment;

const PLATFORMS: [Platform; 2] = [Platform::Unix, Platform::Windows];

#[test]
fn an_empty_or_relative_home_is_no_home_at_all() {
    for home in [Path::new(""), Path::new("relative/home"), Path::new(".")] {
        let variables = [("HOME", home), ("USERPROFILE", home), ("APPDATA", home)];
        for platform in PLATFORMS {
            let env = environment(&variables);
            assert_eq!(platform.home(&env), None, "{platform:?} {home:?}");
            assert_eq!(platform.dir(&env), None, "{platform:?} {home:?}");
            assert_eq!(platform.data_dir(&env), None, "{platform:?} {home:?}");
        }
    }
}

#[test]
fn an_absolute_home_holds_the_configuration_and_the_data() {
    let home = TempDir::new().expect("tempdir");
    let variables = [("HOME", home.path())];
    let env = environment(&variables);
    assert_eq!(Platform::Unix.home(&env), Some(home.path().to_owned()));
    assert_eq!(
        Platform::Unix.dir(&env),
        Some(home.path().join(".config").join("ffm"))
    );
    assert_eq!(
        Platform::Unix.data_dir(&env),
        Some(home.path().join(".local").join("share").join("ffm"))
    );
}

#[test]
fn the_data_directory_prefers_an_absolute_xdg_data_home() {
    let xdg = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("tempdir");
    for platform in PLATFORMS {
        let absolute = [("XDG_DATA_HOME", xdg.path()), ("HOME", home.path())];
        assert_eq!(
            platform.data_dir(environment(&absolute)),
            Some(xdg.path().join("ffm"))
        );
        let relative = [("XDG_DATA_HOME", Path::new("data")), ("HOME", home.path())];
        assert_eq!(
            platform.data_dir(environment(&relative)),
            Some(home.path().join(".local").join("share").join("ffm"))
        );
    }
}

/// Windows sets no `HOME` of its own, so `%USERPROFILE%` is the default; a
/// `HOME` the user did set (Git Bash, MSYS2, a dotfiles setup) is the
/// deliberate choice and wins, as it did before the home rule was shared.
#[test]
fn windows_prefers_an_explicit_home_over_userprofile() {
    let profile = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("tempdir");
    let both = [("USERPROFILE", profile.path()), ("HOME", home.path())];
    assert_eq!(
        Platform::Windows.home(environment(&both)),
        Some(home.path().to_owned())
    );
    assert_eq!(
        Platform::Windows.home(environment(&[("USERPROFILE", profile.path())])),
        Some(profile.path().to_owned())
    );
    assert_eq!(
        Platform::Windows.home(environment(&[("HOME", home.path())])),
        Some(home.path().to_owned())
    );
    assert_eq!(
        Platform::Unix.home(environment(&[("USERPROFILE", profile.path())])),
        None
    );
}

/// The process's own rule reads the process environment: an empty `HOME=`
/// is no home on Unix.
#[cfg(unix)]
#[test]
fn user_home_reads_the_process_environment() {
    let home = TempDir::new().expect("tempdir");
    {
        let _home = crate::test_support::EnvVarGuard::set("HOME", home.path());
        assert_eq!(super::super::user_home(), Some(home.path().to_owned()));
    }
    let _empty = crate::test_support::EnvVarGuard::set("HOME", Path::new(""));
    assert_eq!(super::super::user_home(), None);
}
