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
    assert_eq!(calls, ["update", "upgrade"]);
    let plan = UpdatePlan {
        manager: Manager::Chocolatey,
        ..plan
    };
    assert_eq!(plan.description(), "choco upgrade all");
    assert!(plan
        .execute_with(|_, _| Ok(3010))
        .expect("reboot")
        .contains("restart"));
}
