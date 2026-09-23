//! Unit tests for root confinement.

use super::*;
use std::ffi::OsString;
use std::fs;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tempfile::TempDir;

/// Roots are always canonical — `/tmp` itself is a symlink on some platforms.
fn root_of(dir: &TempDir) -> PathBuf {
    dir.path().canonicalize().expect("canonicalize temp dir")
}

/// `resolve_root` reads process-global state, so the tests that drive it take
/// turns on this lock.
static ROOT_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Holds the lock for one test and puts `$FFM_MCP_ROOT` and the working
/// directory back, so a later test never inherits the previous one's setup.
struct RootEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous_root: Option<OsString>,
    previous_cwd: PathBuf,
}

impl RootEnvGuard {
    fn acquire() -> Self {
        Self {
            // A test that panicked while holding the lock left the environment
            // restored by this guard's `Drop`, so the poison carries no risk.
            _lock: ROOT_ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner),
            previous_root: std::env::var_os(ROOT_ENV),
            previous_cwd: std::env::current_dir().expect("current directory"),
        }
    }
}

impl Drop for RootEnvGuard {
    fn drop(&mut self) {
        match &self.previous_root {
            Some(value) => std::env::set_var(ROOT_ENV, value),
            None => std::env::remove_var(ROOT_ENV),
        }
        if let Err(error) = std::env::set_current_dir(&self.previous_cwd) {
            eprintln!("could not restore the working directory: {error}");
        }
    }
}

#[test]
fn accepts_path_inside_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::write(root.join("inside.txt"), "hi").expect("write file");

    let resolved = sanitize_path(&root, &root.join("inside.txt").display().to_string())
        .expect("path inside the root must be accepted");
    assert_eq!(resolved, root.join("inside.txt"));
}

#[test]
fn accepts_the_root_itself() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let resolved = sanitize_path(&root, &root.display().to_string()).expect("root is in root");
    assert_eq!(resolved, root);
}

#[test]
fn relative_paths_resolve_against_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::write(root.join("rel.txt"), "hi").expect("write file");

    let resolved = sanitize_path(&root, "rel.txt").expect("relative path inside the root");
    assert_eq!(resolved, root.join("rel.txt"));
}

#[test]
fn rejects_absolute_path_outside_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("outside temp dir");
    let root = root_of(&dir);
    let outside_file = outside.path().join("passwd");
    fs::write(&outside_file, "not really a password file").expect("write outside fixture");

    let error = sanitize_path(&root, &outside_file.display().to_string())
        .expect_err("absolute escape must be rejected");
    assert!(
        error.contains("outside the allowed root"),
        "error should name the guard, got: {error}"
    );
    assert!(
        error.contains(&root.display().to_string()),
        "error should name the root so an operator can diagnose it, got: {error}"
    );
}

#[test]
fn rejects_parent_directory_escape() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    sanitize_path(&root, "../..").expect_err("'../..' must not escape the root");
    let outside = root.join("sub").join("..").join("..");
    sanitize_path(&root, &outside.display().to_string())
        .expect_err("traversal through an existing dir must not escape the root");
}

#[test]
fn rejects_parent_escape_for_new_files() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // Parent does not exist, so only lexical normalization can catch this one.
    sanitize_path_create(&root, "new/../../evil.txt")
        .expect_err("create must not escape the root either");
}

#[test]
fn accepts_new_file_inside_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let resolved = sanitize_path_create(&root, "deep/nested/file.txt").expect("new file in root");
    assert_eq!(resolved, root.join("deep/nested/file.txt"));
}

#[cfg(unix)]
#[test]
fn rejects_symlink_pointing_out_of_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let target = root_of(&outside).join("secret.txt");
    fs::write(&target, "secret").expect("write file");

    let link = root.join("escape");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    let error = sanitize_path(&root, &link.display().to_string())
        .expect_err("a symlink out of the root must be rejected");
    assert!(error.contains("outside the allowed root"), "got: {error}");

    // The link itself lives inside the root, so a no-follow resolution is fine:
    // that is what lets `delete` remove the link without touching the target.
    let resolved = sanitize_path_nofollow(&root, &link.display().to_string())
        .expect("the link path itself is inside the root");
    assert_eq!(resolved, link);
}

/// Regression: `sanitize_path_create` used to confine a purely lexical path
/// whenever the parent did not exist, so an intermediate symlink out of the
/// root was never followed and `starts_with(root)` passed trivially. The caller
/// then ran `create_dir_all` + `fs::write` on the far side of the link.
#[cfg(unix)]
#[test]
fn create_rejects_a_new_path_below_a_symlink_out_of_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let outside_root = root_of(&outside);
    std::os::unix::fs::symlink(&outside_root, root.join("data")).expect("symlink");

    // Both entry points must refuse it — the read side already did, the create
    // side is the one that leaked.
    let read_error =
        sanitize_path(&root, "data/x.txt").expect_err("reads through the link must be refused");
    assert!(
        read_error.contains("outside the allowed root"),
        "{read_error}"
    );

    let error = sanitize_path_create(&root, "data/newdir/x.txt")
        .expect_err("a new path below a symlink out of the root must be refused");
    assert!(error.contains("outside the allowed root"), "got: {error}");
    assert!(
        fs::symlink_metadata(outside_root.join("newdir")).is_err(),
        "nothing may be created outside the root"
    );
}

/// Regression: a *dangling* final symlink is invisible to `path.exists()`, so
/// the old code returned `<root>/link` — a path that passes `confine` while
/// `fs::write` on it follows the link to a target the guard never saw.
#[cfg(unix)]
#[test]
fn create_refuses_to_write_through_a_dangling_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    let target = root_of(&outside).join("victim.txt");
    let link = root.join("link");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    let error = sanitize_path_create(&root, "link")
        .expect_err("a dangling link must not be written through");
    assert!(error.contains("dangling symlink"), "got: {error}");

    // `symlink_metadata`, never `exists()`: `exists()` follows the link and
    // would make both assertions vacuous.
    assert!(
        fs::symlink_metadata(&link)
            .expect("the link itself is still there")
            .file_type()
            .is_symlink(),
        "the link must be left untouched"
    );
    assert!(
        fs::symlink_metadata(&target).is_err(),
        "the link target must not have been created outside the root"
    );
}

/// Regression: the lexical branch compared an *unresolved* path against the
/// *canonical* root, so a root reached through a symlink (`/home -> /mnt/home`,
/// macOS `/var/folders`) rejected every path whose parents did not exist yet —
/// while the same path with existing parents was accepted.
#[cfg(unix)]
#[test]
fn create_accepts_new_parents_when_the_root_is_reached_through_a_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let real = dir.path().join("real");
    fs::create_dir_all(real.join("proj")).expect("mkdir");
    let via_link = dir.path().join("home");
    std::os::unix::fs::symlink(&real, &via_link).expect("symlink");

    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, via_link.join("proj"));
    let root = resolve_root().expect("the symlinked root resolves");
    fs::write(root.join("existing.txt"), "hi").expect("write file");

    let existing = via_link.join("proj/existing.txt").display().to_string();
    assert_eq!(
        sanitize_path_create(&root, &existing).expect("an existing file was always accepted"),
        root.join("existing.txt")
    );

    let fresh = via_link.join("proj/deep/new/f.txt").display().to_string();
    assert_eq!(
        sanitize_path_create(&root, &fresh)
            .expect("a path under the symlinked root is inside the root"),
        root.join("deep/new/f.txt"),
        "both branches must compare canonical to canonical"
    );
}

#[cfg(unix)]
#[test]
fn nofollow_rejects_escape_through_the_parent() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let link = root.join("out");
    std::os::unix::fs::symlink(root_of(&outside), &link).expect("symlink");

    let error = sanitize_path_nofollow(&root, &link.join("file.txt").display().to_string())
        .expect_err("the parent must be confined too");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}

#[test]
fn rejects_empty_and_null_paths_before_anything_else() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    assert!(sanitize_path(&root, "")
        .expect_err("empty")
        .contains("empty"));
    assert!(sanitize_path(&root, "a\0b")
        .expect_err("null byte")
        .contains("null byte"));
}

#[test]
fn rejects_missing_parent_for_read_operations() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let error = sanitize_path(&root, "no/such/dir/file.txt").expect_err("missing parent");
    assert!(error.contains("parent directory does not exist"), "{error}");
}

// ─── resolve_root ───────────────────────────────────────────────

#[test]
fn resolve_root_prefers_the_environment_variable() {
    let dir = TempDir::new().expect("temp dir");
    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, dir.path());

    assert_eq!(
        resolve_root().expect("the configured root resolves"),
        root_of(&dir),
        "{ROOT_ENV} must decide the sandbox, not the working directory"
    );
}

#[test]
fn resolve_root_falls_back_to_the_working_directory() {
    let dir = TempDir::new().expect("temp dir");
    let _guard = RootEnvGuard::acquire();
    std::env::remove_var(ROOT_ENV);
    std::env::set_current_dir(dir.path()).expect("change working directory");

    assert_eq!(
        resolve_root().expect("the working directory is the fallback root"),
        root_of(&dir)
    );
}

#[test]
fn resolve_root_fails_closed_when_the_configured_root_is_missing() {
    let dir = TempDir::new().expect("temp dir");
    let missing = dir.path().join("no_such_root");
    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, &missing);

    // Falling back to the cwd here would silently hand the server a sandbox
    // the operator never asked for, so a bad root has to abort startup.
    let error = resolve_root().expect_err("a missing root must not resolve to anything");
    assert!(
        error.contains(ROOT_ENV),
        "error should name the variable, got: {error}"
    );
    assert!(
        error.contains(&missing.display().to_string()),
        "error should name the unusable path, got: {error}"
    );
}

#[cfg(unix)]
#[test]
fn resolve_root_fails_closed_when_the_working_directory_is_gone() {
    let dir = TempDir::new().expect("temp dir");
    let gone = dir.path().to_path_buf();
    let _guard = RootEnvGuard::acquire();
    std::env::remove_var(ROOT_ENV);
    std::env::set_current_dir(&gone).expect("change working directory");
    // The directory the process sits in is removed underneath it, so even the
    // fallback has nothing to resolve. Guessing a root here would be worse
    // than refusing to start. The guard restores the previous cwd on drop.
    fs::remove_dir(&gone).expect("remove the working directory");

    let error = resolve_root().expect_err("there is no directory left to fall back to");
    assert!(
        error.contains("cannot determine current directory"),
        "got: {error}"
    );
}

// ─── canonicalize / invalid paths ───────────────────────────────

#[test]
fn nofollow_rejects_a_path_under_a_missing_directory() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // No-follow keeps the last component verbatim, so only the parent gets
    // canonicalized — and a parent that is not there has to be an error, not
    // an unchecked path handed on to the filesystem.
    let error = sanitize_path_nofollow(&root, "no_such_dir/file.txt")
        .expect_err("the parent cannot be resolved");
    assert!(error.contains("cannot resolve path"), "got: {error}");
    assert!(
        error.contains("no_such_dir"),
        "the error should name the path that failed, got: {error}"
    );
}

#[test]
fn rejects_a_missing_path_that_has_no_final_component() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // "missing/.." does not exist and ends in `..`, so there is no name to
    // create — accepting it would mean creating something at the parent.
    let error =
        sanitize_path_create(&root, "missing/..").expect_err("there is no file name to create");
    assert!(error.contains("invalid path"), "got: {error}");
}

// ─── confine ────────────────────────────────────────────────────

#[test]
fn rejects_a_sibling_that_only_shares_the_root_name_prefix() {
    let base_dir = TempDir::new().expect("temp dir");
    let base = base_dir
        .path()
        .canonicalize()
        .expect("canonicalize temp dir");
    // The classic containment bug: a textual `starts_with` says `/srv/data` is
    // a prefix of `/srv/database`, and the whole sibling tree leaks out.
    let root = base.join("data");
    let sibling = base.join("database");
    fs::create_dir(&root).expect("mkdir root");
    fs::create_dir(&sibling).expect("mkdir sibling");
    let secret = sibling.join("secret.txt");
    fs::write(&secret, "secret").expect("write file");

    let error = sanitize_path(&root, &secret.display().to_string())
        .expect_err("a name-prefix sibling is not inside the root");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}

// ─── sanitize_path_nofollow ─────────────────────────────────────

#[test]
fn nofollow_accepts_a_path_that_ends_in_a_parent_component() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);
    fs::create_dir(root.join("sub")).expect("mkdir");

    // `sub/..` has no final component to keep verbatim, so the no-follow path
    // has to fall back to canonicalizing the whole thing.
    let resolved = sanitize_path_nofollow(&root, "sub/..").expect("resolves back to the root");
    assert_eq!(resolved, root);
}

#[test]
fn nofollow_rejects_the_filesystem_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    let error =
        sanitize_path_nofollow(&root, "/").expect_err("'/' has no final component and is outside");
    assert!(error.contains("outside the allowed root"), "got: {error}");
}

// ─── normalize_lexically ────────────────────────────────────────

#[test]
fn normalize_lexically_drops_current_dir_components() {
    assert_eq!(
        normalize_lexically(Path::new("./a/b")),
        PathBuf::from("a/b")
    );
}

#[test]
fn normalize_lexically_folds_parent_components() {
    assert_eq!(normalize_lexically(Path::new("a/../b")), PathBuf::from("b"));
    assert_eq!(
        normalize_lexically(Path::new("/a/b/../c")),
        PathBuf::from("/a/c")
    );
}

#[test]
fn normalize_lexically_collapses_repeated_and_trailing_slashes() {
    assert_eq!(
        normalize_lexically(Path::new("/a//b/")),
        PathBuf::from("/a/b")
    );
}

#[test]
fn normalize_lexically_cannot_pop_past_the_filesystem_root() {
    // More `..` than components: the result must stay at `/` so that `confine`
    // still sees an absolute path to reject rather than a relative fragment.
    assert_eq!(
        normalize_lexically(Path::new("/a/../../..")),
        PathBuf::from("/")
    );
}

#[test]
fn create_folds_parent_components_that_stay_inside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let root = root_of(&dir);

    // Nothing on disk to canonicalize, so only the lexical fold can decide —
    // and it must accept a `..` that never leaves the root.
    let resolved = sanitize_path_create(&root, "new/deep/../file.txt")
        .expect("a '..' that stays inside the root is allowed");
    assert_eq!(resolved, root.join("new").join("file.txt"));
}
