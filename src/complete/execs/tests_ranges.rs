//! Purpose: Sorted prefix ranges include Unicode and the edges of the list.

use super::matching_in;

fn names() -> Vec<String> {
    [
        "Cargo",
        "cargo",
        "cargo-clippy",
        "cargo-fmt",
        "z",
        "ёж",
        "ёжик",
    ]
    .map(str::to_owned)
    .into()
}

#[test]
fn sorted_prefix_ranges_preserve_all_matches_and_order() {
    let names = names();
    for prefix in [
        "",
        "A",
        "Cargo",
        "c",
        "cargo",
        "cargo-fmt",
        "cargo-z",
        "z",
        "zz",
        "ё",
        "ёжик",
        "я",
    ] {
        let expected: Vec<String> = names
            .iter()
            .filter(|name| !prefix.is_empty() && name.starts_with(prefix))
            .cloned()
            .collect();
        assert_eq!(matching_in(&names, prefix), expected, "{prefix}");
    }
    assert_eq!(matching_in(&[], "cargo"), Vec::<String>::new());
}

#[test]
fn warm_and_executables_populate_ready_and_support_matching() {
    let temp = tempfile::TempDir::new().unwrap();
    let executable = temp.path().join("ffm-fixture-tool");
    std::fs::write(&executable, b"fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--ignored",
            "--exact",
            "complete::execs::tests_ranges::warm_with_private_path",
        ])
        .env("PATH", temp.path());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    assert!(child.status().unwrap().success());
}

#[test]
#[ignore = "subprocess fixture: the parent test supplies an isolated PATH"]
fn warm_with_private_path() {
    super::warm();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while super::ready().is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "PATH scan did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let list = super::executables();
    assert_eq!(list, ["ffm-fixture-tool"]);
    assert_eq!(super::ready().unwrap(), list);
    assert_eq!(super::matching("ffm-fixture"), list);
    assert_eq!(super::matching(""), Vec::<String>::new());
    assert_eq!(
        super::matching("__ffm_nonexistent_exec_99__"),
        Vec::<String>::new()
    );
}

#[cfg(windows)]
#[test]
fn windows_scan_follows_file_symlinks_and_skips_dir_or_dangling_symlinks() {
    let temp = tempfile::TempDir::new().unwrap();
    let bin = temp.path().join("bin");
    let target_dir = temp.path().join("sub");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();
    let real_exe = temp.path().join("real.exe");
    std::fs::write(&real_exe, b"MZ").unwrap();

    if std::os::windows::fs::symlink_file(&real_exe, bin.join("linked.exe")).is_err() {
        return;
    }
    drop(std::os::windows::fs::symlink_dir(
        &target_dir,
        bin.join("linked_dir.exe"),
    ));
    drop(std::os::windows::fs::symlink_file(
        temp.path().join("missing.exe"),
        bin.join("dangling.exe"),
    ));
    let path_var = super::join_paths([bin.as_path()]).unwrap();
    assert_eq!(super::scan(Some(&path_var)), vec!["linked.exe"]);
}
