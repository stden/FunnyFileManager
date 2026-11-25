//! Purpose: Cache-capacity eviction and subprocess failure/timeout fallback tests
//! for git-ignore probing.

use std::process::Command;

use tempfile::TempDir;

use super::tests::repository;

#[test]
fn ignore_cache_evicts_oldest_directory_at_capacity() {
    let outside = TempDir::new().expect("outside");
    let stamp =
        super::cache::IgnoreStamp::capture(outside.path(), outside.path(), outside.path(), &[]);
    for i in 0..=256 {
        super::cache::store(&outside.path().join(format!("d{i}")), stamp, &[]);
    }
    assert_eq!(
        super::cache::lookup(&outside.path().join("d255"), stamp),
        Some(Vec::new())
    );
    assert_eq!(
        super::cache::lookup(&outside.path().join("d256"), stamp),
        Some(Vec::new())
    );
}

#[test]
fn failed_spawn_and_hung_check_ignore_fall_back_cleanly() {
    let repo = repository();
    let invalid_dir = std::path::Path::new("bad\0dir");
    assert_eq!(
        super::ignored_names(
            invalid_dir,
            repo.path(),
            &repo.path().join(".git"),
            &[("a.log", false)]
        ),
        Vec::<String>::new()
    );

    #[cfg(unix)]
    let mut child = Command::new("sleep")
        .arg("10")
        .spawn()
        .expect("spawn sleep");
    #[cfg(windows)]
    let mut child = Command::new("ping.exe")
        .args(["-n", "6", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn ping");
    assert_eq!(
        super::wait_for_exit(&mut child, std::time::Duration::from_millis(20)),
        None
    );
}
