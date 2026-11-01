//! Purpose: Writing a config: created, added to, updated, left alone, and never over a newer file.

use super::*;

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
    assert_eq!(registration.ran.len(), 0);
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
    assert_eq!(fixture.machine.files().unwrap().len(), 0);
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
