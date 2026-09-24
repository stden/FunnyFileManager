//! Unit tests for [`super`]: added, updated and unchanged files, symlinks, new
//! folders, and the failures that must leave a file exactly as it was.

use crate::mcp_clients::{Blocker, ClientId, ProjectDir, Scope};
use crate::test_support::{file_identity, FakeMachine};

use super::super::survey::project_dir;
use super::*;

struct Fixture {
    machine: FakeMachine,
    server: ServerBinary,
    folder: ProjectDir,
}

fn fixture() -> Fixture {
    let machine = FakeMachine::new().expect("fake machine");
    let server = ServerBinary::new(machine.install_server().unwrap()).unwrap();
    let folder = project_dir(&machine.folder()).unwrap();
    Fixture {
        machine,
        server,
        folder,
    }
}

impl Fixture {
    fn register(&self, client: ClientId, target: Target<'_>) -> Registration {
        register(&self.machine.host(), client.client(), target, &self.server)
    }
}

/// The command the `ffm-mcp` entry of a JSON config starts, read as JSON:
/// the file escapes a Windows path's backslashes, so the path itself is not
/// in its text.
fn json_command(text: &str) -> Option<String> {
    let config: serde_json::Value = serde_json::from_str(text).ok()?;
    let command = config.pointer("/mcpServers/ffm-mcp/command")?.as_str()?;
    Some(command.to_owned())
}

/// The folder the `ffm-mcp` entry of Codex's TOML config confines the
/// server to, read as TOML rather than found in the text.
fn toml_root(text: &str) -> Option<String> {
    let config: toml_edit::DocumentMut = text.parse().ok()?;
    let root = config
        .get("mcp_servers")?
        .get("ffm-mcp")?
        .get("env")?
        .get("FFM_MCP_ROOT")?
        .as_str()?;
    Some(root.to_owned())
}

#[test]
fn a_missing_file_is_created_with_its_folders() {
    let fixture = fixture();
    let registration = fixture.register(ClientId::Kiro, Target::AllProjects);
    let path = fixture.machine.config(ClientId::Kiro, Scope::AllProjects);
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(registration.config, Some(path.clone()));
    assert!(registration.ran.is_empty());
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(
        json_command(&text).as_deref(),
        Some(fixture.server.as_str()),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn a_new_file_is_readable_by_its_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture();
    fixture.register(ClientId::Junie, Target::AllProjects);
    let path = fixture.machine.config(ClientId::Junie, Scope::AllProjects);
    let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn an_unconfigured_file_is_added_to() {
    let fixture = fixture();
    fixture
        .machine
        .write_config(ClientId::Junie, Scope::AllProjects, r#"{"theme": "dark"}"#)
        .unwrap();
    let registration = fixture.register(ClientId::Junie, Target::AllProjects);
    assert_eq!(registration.outcome, Outcome::Added);
}

#[test]
fn a_different_entry_is_updated_with_the_folder_as_its_root() {
    let fixture = fixture();
    let text = "[mcp_servers.ffm-mcp]\ncommand = \"/old\"\n";
    let path = fixture
        .machine
        .write_config(ClientId::Codex, Scope::CurrentFolder, text)
        .unwrap();
    let registration = fixture.register(ClientId::Codex, Target::CurrentFolder(&fixture.folder));
    assert_eq!(registration.outcome, Outcome::Updated);
    let codex = fs::read_to_string(path).unwrap();
    assert_eq!(
        toml_root(&codex).as_deref(),
        Some(fixture.folder.as_str()),
        "{codex}"
    );
}

/// The file itself is compared, not its time: an atomic rewrite within one
/// tick of the clock keeps the time and changes the inode (review finding).
#[test]
fn a_second_run_leaves_the_bytes_and_the_file_alone() {
    let fixture = fixture();
    fixture.register(ClientId::Kiro, Target::AllProjects);
    let path = fixture.machine.config(ClientId::Kiro, Scope::AllProjects);
    let before = (fs::read(&path).unwrap(), file_identity(&path).unwrap());
    let again = fixture.register(ClientId::Kiro, Target::AllProjects);
    let after = (fs::read(&path).unwrap(), file_identity(&path).unwrap());
    assert_eq!(again.outcome, Outcome::Unchanged);
    assert_eq!(before, after);
}

#[cfg(unix)]
#[test]
fn an_update_is_a_new_file_the_identity_check_can_see() {
    let fixture = fixture();
    let path = fixture
        .machine
        .write_config(ClientId::Kiro, Scope::AllProjects, "{}")
        .unwrap();
    let before = file_identity(&path).unwrap();
    fixture.register(ClientId::Kiro, Target::AllProjects);
    assert_ne!(file_identity(&path).unwrap(), before);
}

/// Review finding: Current folder at the home folder wrote the user-level
/// configs, and every project was then confined to the home.
#[test]
fn the_home_folder_as_current_folder_writes_nothing() {
    let fixture = fixture();
    let home = ProjectDir::new(fixture.machine.home()).unwrap();
    for client in [ClientId::Codex, ClientId::Junie, ClientId::Kiro] {
        let registration = fixture.register(client, Target::CurrentFolder(&home));
        assert_eq!(
            registration.outcome,
            Outcome::Failed(Failure::Blocked(Blocker::SameAsUserConfig)),
            "{client:?}"
        );
        assert_eq!(
            registration.config,
            Some(fixture.machine.config(client, Scope::AllProjects))
        );
    }
    assert!(fixture.machine.files().unwrap().is_empty());
}

/// Review finding: a save by the client between the last read and the
/// rename was thrown away. The file is compared once more right before the
/// rename, and a change stops it.
#[test]
fn a_file_that_changed_since_it_was_read_is_not_replaced() {
    let fixture = fixture();
    let path = fixture
        .machine
        .write_config(
            ClientId::Kiro,
            Scope::AllProjects,
            r#"{"saved": "by Kiro"}"#,
        )
        .unwrap();
    let error = replace(&path, Some("{}"), "{\"mcpServers\": {}}\n").unwrap_err();
    assert!(
        error.to_string().contains("changed while being written"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        r#"{"saved": "by Kiro"}"#
    );
    assert_eq!(dir_entries(&path), 1, "no temp file left behind");

    let new = fixture.machine.folder().join(".mcp.json");
    fs::write(&new, "{}").unwrap();
    let error = replace(&new, None, "{}\n").unwrap_err();
    assert!(error.to_string().contains("changed"), "{error}");
}

/// How many entries the folder holding `path` has.
fn dir_entries(path: &Path) -> usize {
    fs::read_dir(path.parent().unwrap()).unwrap().count()
}

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
