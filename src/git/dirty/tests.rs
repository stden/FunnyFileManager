//! Unit tests for sanitized, bounded `git status` probing and cache inspection.

use std::ffi::OsStr;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use super::probe::{status_command, wait_for_status};
use super::*;
use crate::models::EntryKind;

#[test]
fn status_command_disables_fsmonitor_and_strips_git_env_overrides() {
    let temp = TempDir::new().expect("temp dir");
    let stdout = tempfile::tempfile().expect("temp stdout");
    let command = status_command(temp.path(), stdout);

    let args: Vec<&OsStr> = command.get_args().collect();
    assert!(
        args.windows(2)
            .any(|pair| pair == [OsStr::new("-c"), OsStr::new("core.fsmonitor=false")]),
        "core.fsmonitor must be disabled to prevent untrusted hook execution: {args:?}"
    );

    let envs: Vec<(&OsStr, Option<&OsStr>)> = command.get_envs().collect();
    assert!(envs.contains(&(OsStr::new("GIT_TERMINAL_PROMPT"), Some(OsStr::new("0")))));
    for removed in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
    ] {
        assert!(
            envs.contains(&(OsStr::new(removed), None)),
            "{removed} must be explicitly removed from child environment"
        );
    }
}

#[test]
fn is_dirty_repo_does_not_execute_core_fsmonitor_hook() {
    let dir = TempDir::new().expect("temp dir");
    if !git_init(dir.path()) {
        return;
    }
    let marker = dir.path().join("pwned");
    let config_path = dir.path().join(".git/config");
    let existing = fs::read_to_string(&config_path).unwrap_or_default();
    let config = format!(
        "{existing}\n[core]\n\tfsmonitor = touch \"{}\"\n",
        marker.display()
    );
    fs::write(&config_path, config).expect("write config");

    assert!(!is_dirty_repo(dir.path()));
    assert!(
        !marker.exists(),
        "core.fsmonitor hook must never be executed when probing dirty status"
    );
}

#[test]
fn wait_for_status_times_out_cleanly_on_hung_process() {
    let stdout = tempfile::tempfile().expect("temp stdout");
    #[cfg(unix)]
    let mut child = Command::new("sh")
        .args(["-c", "sleep 10"])
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().expect("clone"))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sleep");
    #[cfg(windows)]
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().expect("clone"))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sleep");

    let dirty = wait_for_status(&mut child, &stdout, Duration::from_millis(30));
    assert!(!dirty, "timed-out silent process should report clean/false");
}

#[test]
fn inspect_dirty_repos_defers_uncached_repos_until_probed_and_reuses_cache() {
    let parent = TempDir::new().expect("temp parent");
    create_merge_repo(&parent.path().join("merge-repo"));
    let uncached = parent.path().join("uncached-repo");
    if !git_init(&uncached) {
        return;
    }
    fs::write(uncached.join("untracked.txt"), "dirty\n").expect("untracked");
    let entries = [dir_row("merge-repo"), dir_row("uncached-repo")];
    let uncached_name = FileName::from("uncached-repo");

    let (dirty, pending) = inspect_dirty_repos(parent.path(), &entries);
    assert_eq!(dirty, BTreeSet::from([FileName::from("merge-repo")]));
    assert_eq!(pending, vec![uncached_name.clone()]);
    assert!(probe_child_repo(parent.path(), &uncached_name));

    let (dirty_cached, pending_cached) = inspect_dirty_repos(parent.path(), &entries);
    assert_eq!(
        dirty_cached,
        BTreeSet::from([FileName::from("merge-repo"), uncached_name.clone()])
    );
    assert_eq!(pending_cached, Vec::<FileName>::new());

    verify_invalidation_and_clean_cache(parent.path(), &uncached, &uncached_name, &entries);
}

#[test]
fn load_dirty_repos_flags_known_repos_and_queues_uncached_ones() {
    let parent = TempDir::new().expect("temp parent");
    create_merge_repo(&parent.path().join("merge-repo"));
    let uncached = parent.path().join("uncached-repo");
    if !git_init(&uncached) {
        return;
    }
    let entries = [dir_row("merge-repo"), dir_row("uncached-repo")];
    let mut metadata = PanelMetadata::default();

    load_dirty_repos(parent.path(), &entries, &mut metadata);

    assert_eq!(
        metadata.dirty_repos,
        BTreeSet::from([FileName::from("merge-repo")])
    );
    assert_eq!(
        metadata.dirty_probe,
        DirtyProbe::Pending(vec![FileName::from("uncached-repo")])
    );
}

fn verify_invalidation_and_clean_cache(
    parent: &Path,
    uncached: &Path,
    uncached_name: &FileName,
    entries: &[DirEntry],
) {
    invalidate_repo_cache_for(uncached);
    let (_, pending) = inspect_dirty_repos(parent, entries);
    assert_eq!(pending, vec![uncached_name.clone()]);

    assert!(!probe_child_repo(parent, &FileName::from("missing")));
    fs::remove_file(uncached.join("untracked.txt")).expect("clean repo");
    assert!(!probe_child_repo(parent, uncached_name));
    assert!(!probe_child_repo(parent, uncached_name));
    let (dirty, pending_clean) = inspect_dirty_repos(parent, entries);
    assert_eq!(dirty, BTreeSet::from([FileName::from("merge-repo")]));
    assert_eq!(pending_clean, Vec::<FileName>::new());
}

fn create_merge_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".git")).expect("merge .git");
    fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    fs::write(repo.join(".git/MERGE_HEAD"), "0123456789abcdef\n").expect("MERGE_HEAD");
}

fn git_init(repo: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .arg(repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn dir_row(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    }
}
