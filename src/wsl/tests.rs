use super::process::{Captured, Exit, StreamOutput};
use super::*;
use crate::models::DistributionName;
use std::time::Duration;

fn success(stdout: impl Into<Vec<u8>>) -> Captured {
    Captured {
        exit: Exit::Success,
        stdout: StreamOutput {
            bytes: stdout.into(),
            truncated: false,
        },
        stderr: StreamOutput::default(),
    }
}

fn distribution(name: &str) -> DistributionName {
    DistributionName::new(name).expect("valid distribution fixture")
}

fn utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

#[test]
fn refresh_reads_names_without_localized_verbose_columns() {
    let result = execute_with(&WslOperation::Refresh, |args, timeout| {
        assert_eq!(args, ["--list", "--quiet"]);
        assert_eq!(timeout, Duration::from_secs(20));
        Ok(success(utf16le(
            "Ubuntu-24.04\r\nDebian\r\nUbuntu-24.04\r\n",
        )))
    })
    .unwrap();
    assert_eq!(
        result,
        WslOutput::Distributions(vec![distribution("Ubuntu-24.04"), distribution("Debian")])
    );
}

#[test]
fn refresh_supports_empty_distribution_list() {
    assert_eq!(
        execute_with(&WslOperation::Refresh, |_, _| Ok(success("\r\n"))).unwrap(),
        WslOutput::Distributions(Vec::new())
    );
}

#[test]
fn refresh_rejects_invalid_names_and_incomplete_lists() {
    for name in ["../Ubuntu", "Ubuntu\\Root", "Ubuntu\tBad", "* Ubuntu"] {
        let error = execute_with(&WslOperation::Refresh, |_, _| Ok(success(name))).unwrap_err();
        assert!(error.contains("invalid distribution"), "{name}: {error}");
    }
    let error = execute_with(&WslOperation::Refresh, |_, _| {
        let mut captured = success("Ubuntu");
        captured.stdout.truncated = true;
        Ok(captured)
    })
    .unwrap_err();
    assert!(error.contains("output limit"));
}

#[test]
fn output_supports_bomless_and_bom_utf16_and_utf8() {
    let names = "Ubuntu\r\nДебиан\r\n";
    let little = utf16le(names);
    let big: Vec<_> = names.encode_utf16().flat_map(u16::to_be_bytes).collect();
    for bytes in [
        names.as_bytes().to_vec(),
        [vec![0xef, 0xbb, 0xbf], names.as_bytes().to_vec()].concat(),
        little.clone(),
        [vec![0xff, 0xfe], little].concat(),
        [vec![0xfe, 0xff], big].concat(),
    ] {
        assert_eq!(decode_text(&bytes, false).unwrap(), names);
    }
}

#[test]
fn malformed_complete_output_is_rejected_but_truncated_display_is_tolerated() {
    for bytes in [
        vec![0xff],
        vec![0xff, 0xfe, 0x41],
        vec![0xff, 0xfe, 0, 0xd8],
    ] {
        assert!(decode_text(&bytes, false).is_err());
    }
    assert_eq!(decode_text(&[b'a', 0xe2, 0x82], true).unwrap(), "a�");
    assert_eq!(
        decode_text(&[0xff, 0xfe, b'a', 0, b'b'], true).unwrap(),
        "a"
    );
}

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

#[test]
fn package_manager_failure_is_an_error_not_a_success_message() {
    let result = execute_with(
        &WslOperation::UpdatePackages(distribution("Custom")),
        |_, _| {
            let mut captured = success("");
            captured.exit = Exit::Failed(Some(127));
            captured.stderr.bytes = b"No supported package manager found.\n".to_vec();
            Ok(captured)
        },
    );
    assert_eq!(
        result.unwrap_err(),
        "Package update for Custom failed (exit code 127).\nNo supported package manager found."
    );
}

#[test]
fn command_failures_keep_stderr_and_exit_code_when_output_is_empty() {
    let result = execute_with(&WslOperation::Refresh, |_, _| {
        let mut captured = success("Details");
        captured.exit = Exit::Failed(Some(1));
        captured.stderr.bytes = utf16le("WSL is not installed.\r\n");
        Ok(captured)
    });
    assert_eq!(
        result.unwrap_err(),
        "WSL distribution refresh failed (exit code 1).\nWSL is not installed.\nDetails"
    );

    let result = execute_with(&WslOperation::Update, |_, _| {
        let mut captured = success("");
        captured.exit = Exit::Failed(Some(42));
        Ok(captured)
    });
    assert!(result.unwrap_err().contains("42"));
}

#[test]
fn runner_failure_is_propagated() {
    assert_eq!(
        execute_with(&WslOperation::Refresh, |_, _| Err(
            "Cannot start WSL".to_owned()
        ))
        .unwrap_err(),
        "Cannot start WSL"
    );
}

#[test]
fn completed_updates_report_empty_and_truncated_output() {
    assert_eq!(
        execute_with(&WslOperation::Update, |_, _| Ok(success(""))).unwrap(),
        WslOutput::Updated("WSL update completed.".to_owned())
    );
    let result = execute_with(
        &WslOperation::UpdatePackages(distribution("Ubuntu")),
        |_, _| {
            let mut captured = success("Installing packages");
            captured.stdout.truncated = true;
            Ok(captured)
        },
    )
    .unwrap();
    let WslOutput::PackagesUpdated(message) = result else {
        panic!("expected package update result");
    };
    assert_eq!(
        message.lines().next(),
        Some("Package update completed: Ubuntu.")
    );
    assert!(message.contains("Installing packages"));
    assert!(message.contains("output truncated; command continued to completion"));
}

#[test]
fn opening_root_never_starts_a_process() {
    let name = distribution("Ubuntu");
    assert_eq!(
        execute_with(&WslOperation::Open(name.clone()), |_, _| panic!(
            "unexpected process"
        ))
        .unwrap(),
        WslOutput::Root(name.root_path())
    );
}

#[cfg(windows)]
#[test]
fn system_backend_can_resolve_a_root_without_executing_wsl() {
    let name = distribution("Debian");
    assert_eq!(
        SystemWslBackend
            .execute(&WslOperation::Open(name.clone()))
            .unwrap(),
        WslOutput::Root(name.root_path())
    );
}

#[cfg(not(windows))]
#[test]
fn system_backend_reports_unavailable_on_other_platforms() {
    for operation in [
        WslOperation::Refresh,
        WslOperation::Update,
        WslOperation::Open(distribution("Ubuntu")),
        WslOperation::UpdatePackages(distribution("Ubuntu")),
    ] {
        assert_eq!(
            SystemWslBackend.execute(&operation).unwrap_err(),
            "WSL is only available on Windows"
        );
    }
}

#[test]
fn completed_updates_with_undecodable_output_are_errors_not_successes() {
    for operation in [
        WslOperation::Update,
        WslOperation::UpdatePackages(distribution("Ubuntu")),
    ] {
        let error = execute_with(&operation, |_, _| Ok(success(vec![0xff]))).unwrap_err();
        assert_eq!(error, "WSL returned invalid text encoding");
    }
}

#[test]
fn a_failure_without_an_exit_code_says_the_status_is_unknown() {
    let result = execute_with(&WslOperation::Update, |_, _| {
        let mut captured = success("");
        captured.exit = Exit::Failed(None);
        Ok(captured)
    });
    assert_eq!(
        result.unwrap_err(),
        "WSL update failed (unknown exit status)."
    );
}
