//! Unit tests: what the prompt is made of.

use super::*;
use std::path::PathBuf;

fn parts(directory: &Path, last: LastCommand) -> Vec<Part> {
    segments(directory, last)
        .into_iter()
        .map(|s| s.part)
        .collect()
}

fn text_of(directory: &Path, part: Part) -> Option<String> {
    segments(directory, LastCommand::Succeeded)
        .into_iter()
        .find(|segment| segment.part == part)
        .map(|segment| segment.text)
}

/// The bullet is the point of the leading segment: it says whether the last
/// command worked, before the user has to read anything.
#[test]
fn the_bullet_reports_the_last_command() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(parts(&here, LastCommand::Succeeded)[0], Part::StatusOk);
    assert_eq!(parts(&here, LastCommand::Failed)[0], Part::StatusFailed);
}

#[test]
fn the_path_is_always_present() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        text_of(&here, Part::Path).as_deref(),
        Some(env!("CARGO_MANIFEST_DIR"))
    );
}

/// A repository shows its branch, in brackets, after the path.
///
/// Built as a fixture rather than relying on this checkout being one: that is
/// an environment fact, and it is false in a copied build tree — which is
/// exactly where `cargo mutants` runs.
#[test]
fn a_repository_shows_its_branch_in_brackets() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    std::fs::write(dir.path().join(".git/HEAD"), "ref: refs/heads/topic\n").expect("HEAD");

    assert_eq!(
        text_of(dir.path(), Part::Branch).as_deref(),
        Some("(topic)")
    );
}

/// …and a directory outside one shows no branch segment at all, rather than
/// empty brackets.
#[test]
fn a_plain_directory_has_no_branch_segment() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    assert!(!parts(dir.path(), LastCommand::Succeeded).contains(&Part::Branch));
}

/// The segments must join back into the line that gets drawn — the renderer
/// measures the pieces, so a gap here is a gap on screen.
#[test]
fn the_segments_join_into_the_prompt() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let joined: String = segments(&here, LastCommand::Succeeded)
        .iter()
        .map(|segment| segment.text.as_str())
        .collect();

    assert!(joined.starts_with('●'), "{joined}");
    assert!(joined.contains(env!("CARGO_MANIFEST_DIR")), "{joined}");
    assert!(joined.ends_with("# ") || joined.ends_with("$ "), "{joined}");
    assert!(!joined.contains("  "), "no double blanks: {joined:?}");
}

/// The sigil says whether this shell can overwrite anything.
#[test]
fn the_sigil_is_hash_for_root_and_dollar_otherwise() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sigil = text_of(&here, Part::Sigil).expect("always present");
    assert!(sigil == "#" || sigil == "$", "{sigil}");
}

/// A known identity, so the assertions below are about the code rather than
/// about whoever happens to be running the suite.
fn someone(user: Option<&str>, host: Option<&str>, privilege: Privilege) -> Identity {
    Identity {
        user: user.map(ToOwned::to_owned),
        host: host.map(ToOwned::to_owned),
        privilege,
    }
}

fn joined(directory: &Path, identity: &Identity) -> String {
    segments_for(directory, LastCommand::Succeeded, identity)
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

/// The whole point of the sigil: `#` means this shell can overwrite anything.
/// Getting it backwards is the most consequential thing this module can do.
#[test]
fn root_gets_a_hash_and_everyone_else_a_dollar() {
    let dir = tempfile::TempDir::new().expect("temp dir");

    let as_root = someone(Some("root"), Some("host"), Privilege::Root);
    assert!(joined(dir.path(), &as_root).ends_with("# "));

    let as_user = someone(Some("ann"), Some("host"), Privilege::User);
    assert!(joined(dir.path(), &as_user).ends_with("$ "));
}

#[test]
fn the_user_and_host_are_shown_as_given() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let identity = someone(Some("ann"), Some("devbox"), Privilege::User);
    let prompt = segments_for(dir.path(), LastCommand::Succeeded, &identity);

    let text = |part: Part| {
        prompt
            .iter()
            .find(|segment| segment.part == part)
            .map(|segment| segment.text.clone())
    };
    assert_eq!(text(Part::User).as_deref(), Some("ann"));
    assert_eq!(text(Part::Host).as_deref(), Some("devbox"));
    assert_eq!(text(Part::At).as_deref(), Some("@"));
    assert!(joined(dir.path(), &identity).contains("ann@devbox"));
}

/// Without a user there is nobody to be `@` anywhere, so the host and the
/// separator go too rather than leaving a dangling `@host`.
#[test]
fn no_user_means_no_host_and_no_separator() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let identity = someone(None, Some("devbox"), Privilege::User);
    let prompt = segments_for(dir.path(), LastCommand::Succeeded, &identity);

    let parts: Vec<Part> = prompt.iter().map(|segment| segment.part).collect();
    assert!(!parts.contains(&Part::User));
    assert!(!parts.contains(&Part::Host));
    assert!(!parts.contains(&Part::At), "no dangling separator");
}

/// A user with no host is shown alone — a login is still worth saying.
#[test]
fn a_user_without_a_host_is_shown_alone() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let identity = someone(Some("ann"), None, Privilege::User);
    let joined = joined(dir.path(), &identity);

    assert!(joined.contains("ann"), "{joined}");
    assert!(
        !joined.contains('@'),
        "no separator with nothing after it: {joined}"
    );
}

/// The bullet must distinguish the outcomes that are failures from the ones
/// that are not. Every status collapsing to one answer makes it decoration.
#[test]
fn only_real_failures_turn_the_bullet_red() {
    use crate::status::Status;

    assert_eq!(
        LastCommand::from_status(&Status::SaveFailed {
            error: "denied".to_owned()
        }),
        LastCommand::Failed
    );
    assert_eq!(
        LastCommand::from_status(&Status::RefreshFailed("gone".to_owned())),
        LastCommand::Failed
    );
    // Not outcomes: a fresh session must not open on a red bullet.
    assert_eq!(
        LastCommand::from_status(&Status::Cleared),
        LastCommand::Succeeded
    );
    assert_eq!(
        LastCommand::from_status(&Status::KeyHints),
        LastCommand::Succeeded
    );
    assert_eq!(
        LastCommand::from_status(&Status::PanelsSwapped),
        LastCommand::Succeeded
    );
}

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
