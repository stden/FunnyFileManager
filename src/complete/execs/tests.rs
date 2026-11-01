//! Purpose: Pins which `$PATH` entries become command-name candidates.

use super::*;
use std::fs;
use tempfile::TempDir;

/// Create `name` inside `dir`, executable or not.
fn make_file(dir: &Path, name: &str, executable: bool) {
    let path = dir.join(name);
    fs::write(&path, b"#!/bin/sh\n").expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if executable { 0o755 } else { 0o644 };
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");
    }
    #[cfg(not(unix))]
    let _ = executable;
}

#[test]
fn only_executable_files_are_offered() {
    let dir = TempDir::new().expect("temp dir");
    make_file(dir.path(), "runnable", true);
    make_file(dir.path(), "readme.txt", false);
    fs::create_dir(dir.path().join("subdir")).expect("mkdir");

    let path_var = join_paths([dir.path()]).expect("PATH");
    let found = scan(Some(&path_var));

    assert!(found.contains(&"runnable".to_owned()));
    assert!(
        !found.contains(&"subdir".to_owned()),
        "a directory on $PATH is not a command"
    );
    // The permission bit is the whole test on Unix; Windows has no execute bit,
    // so every plain file counts there and only the directory is excluded.
    #[cfg(unix)]
    assert!(
        !found.contains(&"readme.txt".to_owned()),
        "a non-executable file is not a command"
    );
}

#[test]
fn names_are_deduplicated_across_directories() {
    let first = TempDir::new().expect("temp dir");
    let second = TempDir::new().expect("temp dir");
    make_file(first.path(), "twice", true);
    make_file(second.path(), "twice", true);

    let path_var = join_paths([first.path(), second.path()]).expect("PATH");
    let found = scan(Some(&path_var));

    assert_eq!(
        found.iter().filter(|name| *name == "twice").count(),
        1,
        "the same command on two $PATH entries is one candidate"
    );
}

#[test]
fn an_unreadable_directory_does_not_stop_the_scan() {
    let good = TempDir::new().expect("temp dir");
    make_file(good.path(), "survivor", true);
    let missing = good.path().join("does-not-exist");

    let path_var = join_paths([missing.as_path(), good.path()]).expect("PATH");
    assert!(scan(Some(&path_var)).contains(&"survivor".to_owned()));
}

#[test]
fn an_unset_path_yields_no_candidates() {
    assert_eq!(scan(None).len(), 0);
}

#[test]
fn the_scan_is_sorted_and_therefore_deterministic() {
    let dir = TempDir::new().expect("temp dir");
    for name in ["zeta", "alpha", "mid"] {
        make_file(dir.path(), name, true);
    }
    let path_var = join_paths([dir.path()]).expect("PATH");
    let found = scan(Some(&path_var));
    let mut sorted = found.clone();
    sorted.sort();
    assert_eq!(found, sorted);
}

#[test]
fn an_empty_prefix_offers_nothing() {
    // Tab at an empty prompt must show the built-in verbs, not all of $PATH.
    assert_eq!(matching("").len(), 0);
}

#[test]
fn matching_is_prefix_based_not_fuzzy() {
    // Whatever the host's $PATH holds, no candidate may come back that does
    // not literally start with the query — a subsequence match here would
    // return most of $PATH for a two-character query.
    for name in matching("l") {
        assert!(name.starts_with('l'), "{name} is not a prefix match");
    }
}

#[test]
#[cfg(unix)]
fn a_symlink_to_a_directory_is_not_a_command() {
    // `DirEntry::metadata` does not follow symlinks and a symlink's own mode is
    // 0o777, so asking the link would call every link on $PATH executable —
    // including the ones pointing at directories.
    let path_dir = TempDir::new().expect("temp dir");
    let target = TempDir::new().expect("temp dir");
    std::os::unix::fs::symlink(target.path(), path_dir.path().join("linkdir")).expect("symlink");

    let path_var = join_paths([path_dir.path()]).expect("PATH");
    assert!(!scan(Some(&path_var)).contains(&"linkdir".to_owned()));
}

#[test]
#[cfg(unix)]
fn a_symlink_to_an_executable_is_a_command() {
    // Most of /usr/bin is symlinks; following them is the point of the fix.
    let path_dir = TempDir::new().expect("temp dir");
    let target = TempDir::new().expect("temp dir");
    make_file(target.path(), "real-tool", true);
    std::os::unix::fs::symlink(
        target.path().join("real-tool"),
        path_dir.path().join("tool"),
    )
    .expect("symlink");

    let path_var = join_paths([path_dir.path()]).expect("PATH");
    assert!(scan(Some(&path_var)).contains(&"tool".to_owned()));
}

#[test]
#[cfg(unix)]
fn a_dangling_symlink_is_not_a_command() {
    let path_dir = TempDir::new().expect("temp dir");
    std::os::unix::fs::symlink(path_dir.path().join("gone"), path_dir.path().join("broken"))
        .expect("symlink");

    let path_var = join_paths([path_dir.path()]).expect("PATH");
    assert!(!scan(Some(&path_var)).contains(&"broken".to_owned()));
}

/// `ready` never blocks: it is `None` until the scan has finished, and
/// `warm` finishes it on a background thread.
#[test]
fn warm_scans_in_the_background_until_ready_answers() {
    warm();
    warm();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let ready = loop {
        if let Some(ready) = ready() {
            break ready;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the background scan finishes"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert_eq!(ready, executables());
}

/// The scan `executables` makes is the one `ready` hands out from then on:
/// the same list, not a second scan.
#[test]
fn ready_answers_with_the_scan_executables_made() {
    let scanned = executables();
    let ready = ready().expect("the scan has run");
    assert!(std::ptr::eq(ready, scanned));
}

/// `matching` is `matching_in` over the process scan: one prefix rule for
/// the command line and the Language servers dialog.
#[test]
fn matching_in_applies_the_same_prefix_rule_to_any_list() {
    let names: Vec<String> = ["pydoc3", "pylsp", "python3", "rust-analyzer"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(matching_in(&names, "py"), ["pydoc3", "pylsp", "python3"]);
    assert_eq!(
        matching_in(&names, "lsp"),
        Vec::<String>::new(),
        "prefix, not fuzzy"
    );
    assert_eq!(matching_in(&names, "").len(), 0);
    assert_eq!(matching("py"), matching_in(executables(), "py"));
}
