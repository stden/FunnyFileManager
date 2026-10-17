//! Purpose: Discovery — which manager is found on `PATH`, and when it goes through sudo.

use tempfile::TempDir;

use super::super::discovery::discover;
use super::*;

#[test]
fn discovery_uses_the_platform_preference_and_requires_an_executable() {
    let temp = TempDir::new().expect("tempdir");
    let brew = temp.path().join("brew");
    std::fs::write(&brew, b"fixture").expect("file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&brew, std::fs::Permissions::from_mode(0o700))
            .expect("executable");
    }
    let plan = discover(&[Manager::Apt, Manager::Brew], &[temp.path().to_owned()]).expect("brew");
    assert_eq!(plan.manager, Manager::Brew);
    assert_eq!(plan.description(), "brew update && brew upgrade");
    assert!(discover(&[Manager::Chocolatey], &[temp.path().to_owned()]).is_none());
}

/// Discovery decides the elevation, so the dialog can show it before anything
/// runs; managers other than apt-get never go through sudo.
#[test]
fn discovery_never_puts_sudo_in_front_of_homebrew() {
    let temp = TempDir::new().expect("tempdir");
    let brew = temp.path().join("brew");
    std::fs::write(&brew, b"fixture").expect("file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&brew, std::fs::Permissions::from_mode(0o700))
            .expect("executable");
    }
    let plan = discover(&[Manager::Brew], &[temp.path().to_owned()]).expect("brew");
    assert_eq!(plan.elevation, Elevation::CurrentUser);
}

/// Only apt-get run by an ordinary user needs sudo: root already has the
/// rights, and brew and choco ask for elevation themselves.
#[cfg(unix)]
#[test]
fn only_apt_run_by_a_user_goes_through_sudo() {
    use crate::prompt::Privilege;

    assert_eq!(
        Elevation::for_manager(Manager::Apt, Privilege::User),
        Elevation::Sudo
    );
    for (manager, privilege) in [
        (Manager::Apt, Privilege::Root),
        (Manager::Brew, Privilege::User),
        (Manager::Brew, Privilege::Root),
        (Manager::Chocolatey, Privilege::User),
    ] {
        assert_eq!(
            Elevation::for_manager(manager, privilege),
            Elevation::CurrentUser,
            "{manager:?} as {privilege:?}"
        );
    }
}
