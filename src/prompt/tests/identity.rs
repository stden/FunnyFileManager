//! Purpose: The identity is read from the real environment — the login name,
//! the host name and the privilege of the running process.

use crate::prompt::identity::{pick_host, pick_user};
use crate::prompt::{Identity, Privilege};

/// `Identity::from_env` must actually read the environment it is running in.
///
/// Asserted as invariants rather than as fixed values, so the test says
/// something true on any machine: if the system knows a login name, so must
/// the prompt; if `/etc/hostname` names the machine, the prompt must name the
/// same one. A reader that returned nothing, or something invented, would pass
/// a test that only checked for non-emptiness.
#[test]
fn the_real_identity_reads_the_real_environment() {
    let identity = Identity::from_env();

    // A blank user or host is worse than none: it renders as `@host` or a
    // stray `@`, so the readers must reject empties rather than pass them on.
    assert!(identity.user.as_ref().is_none_or(|u| !u.is_empty()));
    assert!(identity.host.as_ref().is_none_or(|h| !h.is_empty()));

    if let Some(expected) = ["USER", "LOGNAME", "USERNAME"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
    {
        assert_eq!(
            identity.user.as_deref(),
            Some(expected.as_str()),
            "the login name must come from the environment, not from nowhere"
        );
    }

    if let Ok(contents) = std::fs::read_to_string("/etc/hostname") {
        let expected = contents.trim();
        if !expected.is_empty() {
            assert_eq!(
                identity.host.as_deref(),
                Some(expected),
                "the host must be the one /etc/hostname names"
            );
        }
    }
}

/// An empty variable is worse than an unset one: it renders as a bare `@host`.
#[test]
fn a_blank_candidate_is_not_a_name() {
    let some = |value: &str| Some(value.to_owned());

    assert_eq!(pick_user(&[some("ann")]).as_deref(), Some("ann"));
    assert_eq!(pick_user(&[some(""), some("ann")]).as_deref(), Some("ann"));
    assert_eq!(pick_user(&[None, some("ann")]).as_deref(), Some("ann"));
    assert_eq!(pick_user(&[some("")]), None);
    assert_eq!(pick_user(&[None, None]), None);
    assert_eq!(pick_user(&[]), None);
}

/// The candidates are tried in order, so the first real one wins.
#[test]
fn the_first_usable_candidate_wins() {
    let some = |value: &str| Some(value.to_owned());
    assert_eq!(
        pick_user(&[some("first"), some("second")]).as_deref(),
        Some("first")
    );
}

/// `/etc/hostname` is what the system believes; `$HOSTNAME` is a shell's copy
/// and is often stale or unset, so the file wins.
#[test]
fn the_hostname_file_outranks_the_variable() {
    assert_eq!(
        pick_host(Some("devbox\n"), Some("stale")).as_deref(),
        Some("devbox")
    );
    assert_eq!(
        pick_host(None, Some("from-env")).as_deref(),
        Some("from-env")
    );
    assert_eq!(
        pick_host(Some("  \n"), Some("from-env")).as_deref(),
        Some("from-env")
    );
    assert_eq!(pick_host(None, None), None);
    assert_eq!(pick_host(Some(""), Some("")), None);
}

/// The file is trimmed: `/etc/hostname` ends with a newline, and `devbox\n`
/// drawn into the prompt would break the row.
#[test]
fn the_hostname_is_trimmed() {
    assert_eq!(
        pick_host(Some("  devbox  \n"), None).as_deref(),
        Some("devbox")
    );
}

#[test]
fn the_current_privilege_is_the_process_effective_user() {
    let expected = if crate::platform::is_root() {
        Privilege::Root
    } else {
        Privilege::User
    };
    assert_eq!(Privilege::current(), expected);
}
