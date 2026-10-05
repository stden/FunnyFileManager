//! Purpose: Tests for the WSL and package update commands and the package script.

use super::*;

#[test]
fn platform_update_has_its_own_command_and_timeout() {
    let result = execute_with(&WslOperation::Update, |args, timeout| {
        assert_eq!(args, ["--update"]);
        assert_eq!(timeout, Duration::from_mins(5));
        Ok(success("WSL is up to date.\r\n"))
    })
    .unwrap();
    assert_eq!(
        result,
        WslOutput::Updated("WSL update completed.\nWSL is up to date.".to_owned())
    );
}

#[test]
fn package_update_passes_name_as_data_and_uses_one_constant_script() {
    let name = "Ubuntu; echo marker";
    let result = execute_with(
        &WslOperation::UpdatePackages(distribution(name)),
        |args, timeout| {
            assert_eq!(
                &args[..7],
                [
                    "--distribution",
                    name,
                    "--user",
                    "root",
                    "--exec",
                    "sh",
                    "-lc"
                ]
            );
            assert_eq!(args.len(), 8);
            assert_eq!(args[7], PACKAGE_UPDATE_SCRIPT);
            assert!(!args[7].contains(name));
            assert_eq!(timeout, Duration::from_hours(1));
            Ok(success("Packages upgraded.\n"))
        },
    )
    .unwrap();
    assert_eq!(
        result,
        WslOutput::PackagesUpdated(
            "Package update completed: Ubuntu; echo marker.\nPackages upgraded.".to_owned()
        )
    );
}

#[test]
fn package_script_uses_normal_supported_package_upgrades() {
    for command in [
        "DEBIAN_FRONTEND=noninteractive",
        "apt-get update\n    apt-get upgrade -y",
        "dnf upgrade -y",
        "yum update -y",
        "zypper --non-interactive update",
        "pacman -Syu --noconfirm",
        "apk update\n    apk upgrade",
        "No supported package manager found",
    ] {
        assert!(PACKAGE_UPDATE_SCRIPT.contains(command), "{command}");
    }
    for release_upgrade in [
        "dist-upgrade",
        "full-upgrade",
        "do-release-upgrade",
        "autoremove",
    ] {
        assert!(!PACKAGE_UPDATE_SCRIPT.contains(release_upgrade));
    }
}

/// `sh -lc` reads a CR as part of the word, so a CRLF checkout turns `set -eu`
/// into `set -eu\r` and the update fails before it starts. `include_str!`
/// copies the file byte for byte; only `.gitattributes` keeps it LF on Windows.
#[test]
fn package_script_reaches_the_shell_with_unix_line_endings() {
    assert!(
        !PACKAGE_UPDATE_SCRIPT.contains('\r'),
        "package_update.sh was checked out with CRLF; .gitattributes fixes new checkouts, \
         an old one needs: git rm --cached src/wsl/package_update.sh && git checkout -- src/wsl/package_update.sh"
    );
}

/// The guard above can only fail on a CRLF checkout; this one fails anywhere
/// the rule that prevents it goes missing.
#[test]
fn shell_scripts_are_pinned_to_lf_in_every_checkout() {
    let attributes = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.gitattributes"));
    assert!(
        attributes
            .lines()
            .any(|line| line.trim() == "*.sh text eol=lf"),
        ".gitattributes must keep shell scripts LF"
    );
}
