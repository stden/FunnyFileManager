//! Purpose: A given identity is shown as given — `#` for root, `user@host`,
//! and no dangling separator when a part is missing.

use std::path::Path;

use crate::prompt::{segments_for, Identity, LastCommand, Part, Privilege};

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
