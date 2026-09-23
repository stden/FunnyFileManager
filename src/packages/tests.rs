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
    };
    let mut calls = Vec::new();
    assert!(plan
        .execute_with(|_, args| {
            calls.push(args.join(" "));
            Ok(100)
        })
        .is_err());
    assert_eq!(calls, ["update"]);
    calls.clear();
    plan.execute_with(|_, args| {
        calls.push(args.join(" "));
        Ok(0)
    })
    .expect("success");
    assert_eq!(calls, ["update", "upgrade -y"]);
    let plan = UpdatePlan {
        manager: Manager::Chocolatey,
        ..plan
    };
    assert_eq!(plan.description(), "choco upgrade all -y");
    assert!(plan
        .execute_with(|_, _| Ok(3010))
        .expect("reboot")
        .contains("restart"));
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
