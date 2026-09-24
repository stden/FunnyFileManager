//! Purpose: Configs that cannot be parsed, read or written, and links in the way.

use super::*;

#[test]
fn a_file_that_does_not_parse_is_reported_and_left_untouched() {
    let fixture = fixture();
    let path = fixture
        .machine
        .write_config(ClientId::Kiro, Scope::AllProjects, "{ broken")
        .unwrap();
    let registration = fixture.register(ClientId::Kiro, Target::AllProjects);
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unparsable(_)))
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), "{ broken");
}

#[test]
fn a_file_that_cannot_be_read_is_reported() {
    let fixture = fixture();
    fs::create_dir_all(fixture.machine.config(ClientId::Junie, Scope::AllProjects)).unwrap();
    let registration = fixture.register(ClientId::Junie, Target::AllProjects);
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unreadable(_)))
    ));
}

#[test]
fn an_unknown_home_is_reported_without_a_path() {
    let fixture = fixture();
    let mut host = fixture.machine.host();
    host.home = None;
    let registration = register(
        &host,
        ClientId::Kiro.client(),
        Target::AllProjects,
        &fixture.server,
    );
    assert_eq!(registration.config, None);
    assert_eq!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::NoHomeFolder))
    );
}

#[test]
fn a_parent_that_is_a_file_fails_the_write_and_changes_nothing() {
    let fixture = fixture();
    fs::write(fixture.machine.home().join(".kiro"), "a file").unwrap();
    let before = fixture.machine.files().unwrap();
    let registration = fixture.register(ClientId::Kiro, Target::AllProjects);
    assert!(
        matches!(&registration.outcome, Outcome::Failed(Failure::Io { .. })),
        "{registration:?}"
    );
    assert_eq!(fixture.machine.files().unwrap(), before);
}

#[cfg(unix)]
#[test]
fn a_symlinked_config_keeps_its_link_and_its_target_changes() {
    let fixture = fixture();
    let real = fixture.machine.home().join("dotfiles-mcp.json");
    fs::write(&real, r#"{"theme": "dark"}"#).unwrap();
    let link = fixture.machine.config(ClientId::Kiro, Scope::AllProjects);
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let registration = fixture.register(ClientId::Kiro, Target::AllProjects);
    assert_eq!(registration.outcome, Outcome::Added);
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    let text = fs::read_to_string(&real).unwrap();
    assert!(text.contains("\"theme\""), "{text}");
    assert_eq!(
        json_command(&text).as_deref(),
        Some(fixture.server.as_str()),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn a_read_only_config_fails_the_write_and_is_left_as_it_was() {
    use std::os::unix::fs::PermissionsExt;
    if crate::platform::is_root() {
        return; // root writes through any mode bits
    }
    let fixture = fixture();
    let path = fixture
        .machine
        .write_config(ClientId::Junie, Scope::AllProjects, "{}")
        .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    let registration = fixture.register(ClientId::Junie, Target::AllProjects);
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Io { .. })
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
}

#[cfg(unix)]
#[test]
fn a_dangling_link_fails_the_write() {
    let fixture = fixture();
    let link = fixture.machine.config(ClientId::Kiro, Scope::AllProjects);
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(fixture.machine.home().join("gone.json"), &link).unwrap();
    let registration = fixture.register(ClientId::Kiro, Target::AllProjects);
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Io { .. })
    ));
}

/// A config that cannot be read at the last look is not replaced either.
#[cfg(unix)]
#[test]
fn a_file_unreadable_at_the_last_look_is_not_replaced() {
    let fixture = fixture();
    let folder = fixture.machine.folder().join("folder");
    fs::create_dir(&folder).unwrap();
    let link = fixture.machine.folder().join(".mcp.json");
    std::os::unix::fs::symlink(&folder, &link).unwrap();
    let error = replace(&link, None, "{}\n").unwrap_err();
    assert!(error.to_string().contains("not a regular file"), "{error}");
}
