use super::*;
use tempfile::TempDir;

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

#[test]
fn updates_stop_on_failure_and_never_run_the_host_package_manager() {
    let temp = TempDir::new().expect("tempdir");
    let plan = UpdatePlan {
        manager: Manager::Apt,
        executable: temp.path().join("apt-get"),
        elevation: Elevation::CurrentUser,
    };
    let mut calls = Vec::new();
    assert!(plan
        .execute_with(|_, args| {
            calls.push(args.join(" "));
            Ok(exited(100, None))
        })
        .is_err());
    assert_eq!(calls, ["update"]);
    calls.clear();
    plan.execute_with(|_, args| {
        calls.push(args.join(" "));
        Ok(exited(0, None))
    })
    .expect("success");
    assert_eq!(calls, ["update", "upgrade -y"]);
    let plan = UpdatePlan {
        manager: Manager::Chocolatey,
        ..plan
    };
    assert_eq!(plan.description(), "choco upgrade all -y");
    assert!(plan
        .execute_with(|_, _| Ok(exited(3010, None)))
        .expect("reboot")
        .contains("restart"));
}

fn exited(code: i32, reason: Option<&str>) -> StepOutcome {
    StepOutcome {
        exit: StepExit::Code(code),
        reason: reason.map(str::to_owned),
    }
}

fn apt(elevation: Elevation) -> UpdatePlan {
    UpdatePlan {
        manager: Manager::Apt,
        executable: PathBuf::from("/usr/bin/apt-get"),
        elevation,
    }
}

/// The dialog is the last chance to see what will run; a plan that goes
/// through sudo says so.
#[test]
fn a_plan_that_needs_sudo_shows_it_in_its_description() {
    assert_eq!(
        apt(Elevation::Sudo).description(),
        "sudo apt-get update && sudo apt-get upgrade -y"
    );
    assert_eq!(
        apt(Elevation::CurrentUser).description(),
        "apt-get update && apt-get upgrade -y"
    );
}

/// sudo exits 1 when it refuses (no password, not in sudoers, no terminal);
/// apt-get never ran, so the message must not blame it.
#[test]
fn sudo_refusing_is_reported_as_sudo_with_its_reason() {
    let reason = Some("sudo: a password is required");
    let error = apt(Elevation::Sudo)
        .execute_with(|_, _| Ok(exited(1, reason)))
        .expect_err("sudo refused");
    assert_eq!(
        error.to_string(),
        "sudo did not run apt-get update (exit code 1): sudo: a password is required"
    );
}

#[test]
fn a_package_manager_failure_names_the_manager_and_its_reason() {
    for elevation in [Elevation::Sudo, Elevation::CurrentUser] {
        let error = apt(elevation)
            .execute_with(|_, _| Ok(exited(100, Some("E: Could not get lock"))))
            .expect_err("apt-get failed");
        assert_eq!(
            error.to_string(),
            "apt-get update exited with code 100: E: Could not get lock"
        );
    }
    let silent = apt(Elevation::Sudo)
        .execute_with(|_, _| Ok(exited(1, None)))
        .expect_err("sudo refused without a word");
    assert_eq!(
        silent.to_string(),
        "sudo did not run apt-get update (exit code 1)"
    );
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

/// The menu suspends the TUI and gives the terminal to the package manager, so
/// a command that stops to ask "Do you want to continue? [Y/n]" hangs with no
/// way to answer. Every manager that has a confirmation flag must pass it.
#[test]
fn every_upgrade_runs_unattended() {
    for manager in [Manager::Chocolatey, Manager::Apt, Manager::Brew] {
        let upgrade = manager
            .commands()
            .iter()
            .find(|args| args.contains(&"upgrade"))
            .expect("every manager upgrades something");

        // `brew` has no confirmation flag and never prompts; the other two do.
        let wants_flag = manager != Manager::Brew;
        assert_eq!(
            upgrade.contains(&"-y"),
            wants_flag,
            "{} upgrade args: {upgrade:?}",
            manager.label()
        );
    }
}

/// A step a signal ended (the OOM killer, a SIGTERM from outside) has no exit
/// code. It used to be reported as "exited with code -1", a code Chocolatey
/// really uses for its own errors.
#[test]
fn a_step_ended_by_a_signal_says_so_instead_of_inventing_a_code() {
    let error = apt(Elevation::Sudo)
        .execute_with(|_, _| {
            Ok(StepOutcome {
                exit: StepExit::Signal,
                reason: None,
            })
        })
        .expect_err("a signalled step is a failure");
    assert_eq!(error.to_string(), "apt-get update was stopped by a signal");
}

/// Chocolatey's "nothing to do" is a success, its -1 an error like any other.
#[test]
fn chocolatey_nothing_to_do_passes_and_its_minus_one_fails() {
    let plan = UpdatePlan {
        manager: Manager::Chocolatey,
        ..apt(Elevation::CurrentUser)
    };
    assert_eq!(
        plan.execute_with(|_, _| Ok(exited(CHOCO_NOTHING_TO_DO, None)))
            .expect("nothing to do"),
        "Package update completed"
    );
    let error = plan
        .execute_with(|_, _| Ok(exited(-1, None)))
        .expect_err("-1 is an error");
    assert!(
        error.to_string().ends_with("exited with code -1"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn an_exit_status_becomes_its_code_or_the_signal() {
    use std::os::unix::process::ExitStatusExt as _;

    // A wait status keeps the exit code in its second byte and a signal
    // number in its low bits.
    assert_eq!(
        StepExit::from(ExitStatus::from_raw(3 << 8)),
        StepExit::Code(3)
    );
    assert_eq!(StepExit::from(ExitStatus::from_raw(9)), StepExit::Signal);
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

/// Exit code 1 is sudo's refusal only when sudo ran; without it, it is the
/// manager's own code.
#[test]
fn exit_one_without_sudo_is_the_managers_own_code() {
    let error = apt(Elevation::CurrentUser)
        .execute_with(|_, _| Ok(exited(1, None)))
        .expect_err("apt-get failed");
    assert_eq!(error.to_string(), "apt-get update exited with code 1");
}
