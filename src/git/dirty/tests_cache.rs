//! Purpose: Invalidation must prevent older probes from restoring stale answers.

use super::*;
use tempfile::TempDir;

#[test]
fn invalidation_rejects_a_probe_started_before_it() {
    let repo = TempDir::new().expect("repository");
    let git_dir = repo.path().join(".git");
    fs::create_dir(&git_dir).expect("git directory");
    let stamp = RepoStamp::capture(repo.path(), &git_dir);
    let started = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("clock has a prior instant");

    invalidate_repo_cache_for(repo.path());
    store_cached(repo.path().to_path_buf(), stamp, started, true);

    assert_eq!(fast_or_cached(repo.path(), stamp, Instant::now()), None);
}

#[test]
fn older_probe_cannot_replace_a_newer_answer_after_invalidation() {
    let repo = TempDir::new().expect("repository");
    let git_dir = repo.path().join(".git");
    fs::create_dir(&git_dir).expect("git directory");
    let stamp = RepoStamp::capture(repo.path(), &git_dir);
    let started = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("clock has a prior instant");

    invalidate_repo_cache_for(repo.path());
    store_cached(repo.path().to_path_buf(), stamp, Instant::now(), false);
    store_cached(repo.path().to_path_buf(), stamp, started, true);

    assert_eq!(
        fast_or_cached(repo.path(), stamp, Instant::now()),
        Some(false)
    );
}
