//! Purpose: Tilde expansion of `cd` arguments, with the home-variable fixture it needs.
use super::*;
use std::env;
use std::ffi::OsString;
use std::sync::MutexGuard;

// ─── tilde expansion ──────────────────────────────────────────────────

#[test]
fn expand_tilde_leaves_other_paths_alone() {
    assert_eq!(expand_tilde(Path::new("/abs/x")), PathBuf::from("/abs/x"));
    assert_eq!(expand_tilde(Path::new("rel/x")), PathBuf::from("rel/x"));
    // `~name` is a user reference we do not support — keep it verbatim.
    assert_eq!(expand_tilde(Path::new("~other")), PathBuf::from("~other"));
}

/// RAII swap of the two home-directory variables `expand_tilde` reads.
///
/// Both are removed on construction and restored on drop, under the
/// process-wide env lock — every test in this crate that expands `~` takes the
/// same lock, so a missing `$HOME` can never leak into a parallel test.
struct HomeVars {
    _lock: MutexGuard<'static, ()>,
    home: Option<OsString>,
    userprofile: Option<OsString>,
}

impl HomeVars {
    fn cleared() -> Self {
        let lock = crate::test_support::env_lock();
        let taken = Self {
            _lock: lock,
            home: env::var_os("HOME"),
            userprofile: env::var_os("USERPROFILE"),
        };
        env::remove_var("HOME");
        env::remove_var("USERPROFILE");
        taken
    }
}

impl Drop for HomeVars {
    fn drop(&mut self) {
        restore_var("HOME", self.home.take());
        restore_var("USERPROFILE", self.userprofile.take());
    }
}

fn restore_var(name: &str, value: Option<OsString>) {
    match value {
        Some(value) => env::set_var(name, value),
        None => env::remove_var(name),
    }
}

/// With no home directory to expand against, `~` must survive verbatim. Turning
/// it into a bare relative `marker` would make `cd ~/marker` silently open a
/// subdirectory of the current panel instead.
#[test]
fn expand_tilde_without_any_home_variable_returns_the_path_unchanged() {
    let _guard = HomeVars::cleared();

    assert_eq!(
        expand_tilde(Path::new("~/marker")),
        PathBuf::from("~/marker")
    );
    assert_eq!(expand_tilde(Path::new("~")), PathBuf::from("~"));
}

/// Windows sets `%USERPROFILE%`, not `$HOME`; the fallback is what makes `cd ~`
/// work there, and it is only reachable while `$HOME` is absent.
#[test]
fn expand_tilde_falls_back_to_userprofile_when_home_is_unset() {
    let _guard = HomeVars::cleared();
    env::set_var("USERPROFILE", Path::new("/profiles/dana"));

    assert_eq!(
        expand_tilde(Path::new("~/marker")),
        PathBuf::from("/profiles/dana/marker")
    );
}

#[test]
fn expand_tilde_replaces_the_leading_tilde_component() {
    // Asserted structurally: other tests swap $HOME concurrently, so the exact
    // value is not stable, but an expanded path is always absolute and keeps
    // the rest of the components. The env lock keeps this off the window in
    // which the tests above run without a home directory at all.
    let _lock = crate::test_support::env_lock();
    let expanded = expand_tilde(Path::new("~/marker"));
    assert!(
        expanded.is_absolute(),
        "~ must expand to an absolute path, got {}",
        expanded.display()
    );
    assert!(expanded.ends_with("marker"));
    assert!(!expanded.starts_with("~"));
}
