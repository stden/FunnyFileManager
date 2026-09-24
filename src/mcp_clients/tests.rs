//! Unit tests for [`super`]: the client table, the entry comparison and the
//! words the dialog and `:mcp` share.

use std::collections::BTreeSet;

use super::*;

/// `unix` as an absolute path on this platform: Windows needs a drive.
pub(super) fn absolute(unix: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", unix.replace('/', "\\")))
    } else {
        PathBuf::from(unix)
    }
}

pub(super) fn server() -> ServerBinary {
    ServerBinary::new(absolute("/srv/x/bin/ffm-mcp")).unwrap()
}

pub(super) fn folder() -> ProjectDir {
    ProjectDir::new(absolute("/srv/x/project")).unwrap()
}

pub(super) fn all_projects() -> ServerEntry {
    ServerEntry {
        command: server(),
        root: None,
    }
}

pub(super) fn current_folder() -> ServerEntry {
    ServerEntry {
        command: server(),
        root: Some(folder()),
    }
}

/// The widest line a dialog on an 80-column screen can draw inside its chrome.
const DIALOG_WIDTH: usize = 72;

#[test]
fn the_table_lists_four_clients_with_unique_ids_names_and_programs() {
    assert_eq!(CLIENTS.len(), 4);
    let ids: BTreeSet<ClientId> = CLIENTS.iter().map(|client| client.id).collect();
    let names: BTreeSet<&str> = CLIENTS.iter().map(|client| client.name).collect();
    let programs: Vec<&str> = CLIENTS
        .iter()
        .flat_map(|client| client.programs.iter().copied())
        .collect();
    let unique_programs: BTreeSet<&str> = programs.iter().copied().collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(names.len(), 4);
    assert_eq!(unique_programs.len(), programs.len());
}

#[test]
fn every_client_id_leads_back_to_its_own_row() {
    for client in &CLIENTS {
        assert_eq!(client.id.client(), client);
        assert_eq!(client.id.name(), client.name);
    }
}

#[test]
fn table_paths_are_relative_and_slash_separated() {
    for client in &CLIENTS {
        for path in [client.user_config, client.project_config, client.home_dir] {
            assert!(!path.starts_with('/'), "{path} is absolute");
            assert!(!path.contains('\\'), "{path} uses a backslash");
        }
    }
}

#[test]
fn the_paths_are_the_ones_each_client_documents() {
    let codex = ClientId::Codex.client();
    assert_eq!(codex.user_config, "config.toml");
    assert_eq!(codex.format, ConfigFormat::Toml);
    assert_eq!(
        ClientId::Junie.client().project_config,
        ".junie/mcp/mcp.json"
    );
    assert_eq!(
        ClientId::ClaudeCode.client().mechanism(Scope::AllProjects),
        Mechanism::ClaudeCli
    );
    assert_eq!(
        ClientId::ClaudeCode
            .client()
            .mechanism(Scope::CurrentFolder),
        Mechanism::EditFile
    );
    assert_eq!(
        ClientId::Kiro.client().config(Scope::CurrentFolder),
        ".kiro/settings/mcp.json"
    );
}

#[test]
fn every_note_fits_an_80_column_dialog() {
    for client in &CLIENTS {
        for scope in Scope::ALL {
            if let Some(note) = client.note(scope) {
                let line = format!("{}: {note}", client.name);
                assert!(line.chars().count() <= DIALOG_WIDTH, "{line}");
            }
        }
    }
}

#[test]
fn notes_warn_junie_and_kiro_at_all_projects_and_claude_and_codex_in_a_folder() {
    let noted = |scope| -> Vec<ClientId> {
        CLIENTS
            .iter()
            .filter(|client| client.note(scope).is_some())
            .map(|client| client.id)
            .collect()
    };
    assert_eq!(noted(Scope::AllProjects), [ClientId::Junie, ClientId::Kiro]);
    assert_eq!(
        noted(Scope::CurrentFolder),
        [ClientId::ClaudeCode, ClientId::Codex]
    );
}

#[test]
fn scope_labels_and_the_other_scope() {
    assert_eq!(Scope::default(), Scope::AllProjects);
    assert_eq!(Scope::AllProjects.label(), "All projects");
    assert_eq!(Scope::CurrentFolder.label(), "Current folder");
    assert_eq!(Scope::AllProjects.other(), Scope::CurrentFolder);
    assert_eq!(Scope::CurrentFolder.other(), Scope::AllProjects);
}

#[test]
fn presence_and_state_labels_are_the_shared_words() {
    assert_eq!(Presence::Detected.label(), "detected");
    assert_eq!(Presence::NotDetected.label(), "not detected");
    let labels = [
        (ConfigState::NoConfig, "— no config"),
        (ConfigState::NotConfigured, "⊘ not configured"),
        (ConfigState::DifferentEntry, "≠ different entry"),
        (ConfigState::Registered, "✓ registered"),
    ];
    for (state, label) in labels {
        assert_eq!(state.label(), label);
    }
}

#[test]
fn a_blocked_state_says_why_in_a_few_words() {
    let labels = [
        (Blocker::Unparsable("x".into()), "✗ config does not parse"),
        (
            Blocker::UnexpectedLayout("mcpServers"),
            "✗ unexpected config layout",
        ),
        (Blocker::Unreadable("x".into()), "✗ config unreadable"),
        (Blocker::NeedsClaudeCli, "✗ needs the claude CLI"),
        (
            Blocker::HomeVarNotFolder("CODEX_HOME"),
            "✗ $CODEX_HOME is not a folder",
        ),
        (Blocker::NoHomeFolder, "✗ no home folder"),
    ];
    for (blocker, label) in labels {
        assert_eq!(ConfigState::Blocked(blocker).label(), label);
    }
}

#[test]
fn only_absent_unconfigured_and_different_configs_are_actionable() {
    assert!(ConfigState::NoConfig.is_actionable());
    assert!(ConfigState::NotConfigured.is_actionable());
    assert!(ConfigState::DifferentEntry.is_actionable());
    assert!(!ConfigState::Registered.is_actionable());
    assert!(!ConfigState::Blocked(Blocker::NoHomeFolder).is_actionable());
}

#[test]
fn blockers_explain_themselves_with_the_config_path() {
    let path = "/home/bob/.kiro/settings/mcp.json";
    let unparsable = Blocker::Unparsable("expected value at line 3 column 1".into());
    assert_eq!(
        unparsable.explain(path),
        "/home/bob/.kiro/settings/mcp.json does not parse (expected value at line 3 column 1); \
         left untouched. Fix or remove it, then register again."
    );
    assert_eq!(
        Blocker::UnexpectedLayout("mcpServers").explain(path),
        "/home/bob/.kiro/settings/mcp.json: mcpServers is not an object; left untouched"
    );
    assert!(Blocker::Unreadable("denied".into())
        .explain(path)
        .starts_with("cannot read /home/bob/.kiro"));
    assert!(Blocker::NeedsClaudeCli.explain(path).contains("claude"));
    assert!(Blocker::HomeVarNotFolder("KIRO_HOME")
        .explain(path)
        .contains("$KIRO_HOME"));
    assert!(Blocker::NoHomeFolder.explain(path).contains("home folder"));
}

#[test]
fn a_server_path_must_be_unicode_and_a_folder_must_be_absolute() {
    let binary = absolute("/srv/x/bin/ffm-mcp");
    assert_eq!(server().as_str(), binary.to_str().unwrap());
    assert_eq!(server().as_path(), binary);
    assert_eq!(folder().as_path(), absolute("/srv/x/project"));
    assert_eq!(ProjectDir::new(PathBuf::from("relative/dir")), None);
    assert_eq!(ProjectDir::new(PathBuf::from("s3://bucket")), None);
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_unicode_is_handed_back() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let raw = PathBuf::from(OsString::from_vec(b"/srv/x/\xff/ffm-mcp".to_vec()));
    assert_eq!(ServerBinary::new(raw.clone()), Err(raw.clone()));
    assert_eq!(ProjectDir::new(raw), None);
}

/// Review finding: the panel's canonical path on Windows is `\\?\C:\…`,
/// which went into `FFM_MCP_ROOT` as it was.
#[cfg(windows)]
#[test]
fn a_project_folder_is_kept_in_its_ordinary_spelling() {
    let folder = ProjectDir::new(PathBuf::from(r"\\?\C:\srv\project")).unwrap();
    assert_eq!(folder.as_str(), r"C:\srv\project");
    let share = ProjectDir::new(PathBuf::from(r"\\?\UNC\host\share\p")).unwrap();
    assert_eq!(share.as_str(), r"\\host\share\p");
}

#[test]
fn only_a_found_server_has_a_binary() {
    assert_eq!(ServerState::Found(server()).binary(), Some(&server()));
    assert_eq!(ServerState::Missing.binary(), None);
    assert_eq!(ServerState::NotUnicode(PathBuf::from("/x")).binary(), None);
}

#[test]
fn an_entry_matches_on_command_empty_args_and_root() {
    let found = |command: &str, args, root: Option<&str>| FoundEntry {
        command: Some(command.to_owned()),
        args,
        root: root.map(str::to_owned),
        launch: Launch::AsWritten,
        switch: Switch::On,
    };
    let binary = server();
    let server = binary.as_str();
    let project = folder();
    let project = project.as_str();
    assert!(found(server, Args::Empty, None).matches(&all_projects()));
    assert!(!found(server, Args::Given, None).matches(&all_projects()));
    assert!(!found("/old/ffm-mcp", Args::Empty, None).matches(&all_projects()));
    // A fixed root contradicts "all projects".
    assert!(!found(server, Args::Empty, Some(project)).matches(&all_projects()));
    assert!(found(server, Args::Empty, Some(project)).matches(&current_folder()));
    assert!(!found(server, Args::Empty, Some("/srv/x/other")).matches(&current_folder()));
    assert!(!found(server, Args::Empty, None).matches(&current_folder()));
    let no_command = FoundEntry {
        command: None,
        args: Args::Empty,
        root: None,
        launch: Launch::AsWritten,
        switch: Switch::On,
    };
    assert!(!no_command.matches(&all_projects()));
}

/// Review finding: a `url` or `cwd` entry matched, and a switched-off one
/// read as plain "registered".
#[test]
fn an_entry_started_elsewhere_differs_and_one_switched_off_is_disabled() {
    let entry = |launch, switch| FoundEntry {
        command: Some(server().as_str().to_owned()),
        args: Args::Empty,
        root: None,
        launch,
        switch,
    };
    let desired = all_projects();
    assert!(!entry(Launch::Elsewhere, Switch::On).matches(&desired));
    assert_eq!(
        entry(Launch::Elsewhere, Switch::Off).state(Some(&desired)),
        ConfigState::DifferentEntry
    );
    assert_eq!(
        entry(Launch::AsWritten, Switch::Off).state(Some(&desired)),
        ConfigState::Disabled
    );
    assert_eq!(
        entry(Launch::AsWritten, Switch::On).state(Some(&desired)),
        ConfigState::Registered
    );
}

#[test]
fn state_dispatches_on_the_format() {
    let json = serde_json::json!({"mcpServers": {"ffm-mcp": {"command": server().as_str()}}});
    let json = json.to_string();
    let toml = format!("[mcp_servers.ffm-mcp]\ncommand = {:?}\n", server().as_str());
    assert_eq!(
        state(ConfigFormat::Json, &json, Some(&all_projects())),
        ConfigState::Registered
    );
    assert_eq!(
        state(ConfigFormat::Toml, &toml, Some(&all_projects())),
        ConfigState::Registered
    );
    assert_eq!(
        state(ConfigFormat::Toml, &toml, Some(&current_folder())),
        ConfigState::DifferentEntry
    );
}

/// Review finding: with no server found, a stale entry — even one for a
/// deleted binary — read as "registered".
#[test]
fn with_no_server_to_compare_an_entry_is_only_unverified() {
    let json = r#"{"mcpServers":{"ffm-mcp":{"command":"/old/ffm-mcp"}}}"#;
    assert_eq!(
        state(ConfigFormat::Json, json, None),
        ConfigState::Unverified {
            command: Some("/old/ffm-mcp".to_owned())
        }
    );
    assert_eq!(
        state(
            ConfigFormat::Toml,
            "[mcp_servers.ffm-mcp]\nargs = []\n",
            None
        ),
        ConfigState::Unverified { command: None }
    );
    assert_eq!(
        state(ConfigFormat::Json, "{}", None),
        ConfigState::NotConfigured
    );
}

#[test]
fn upsert_dispatches_on_the_format() {
    let json = upsert(ConfigFormat::Json, "", &all_projects()).unwrap();
    let toml = upsert(ConfigFormat::Toml, "", &all_projects()).unwrap();
    assert!(json.contains("\"mcpServers\""));
    assert!(toml.contains("[mcp_servers.ffm-mcp]"));
}

#[test]
fn a_note_line_names_its_client() {
    assert_eq!(
        ClientId::Kiro
            .client()
            .note_line(Scope::AllProjects)
            .as_deref(),
        Some("Kiro: may start ffm-mcp outside the project; prefer Current folder.")
    );
    assert_eq!(
        ClientId::Kiro.client().note_line(Scope::CurrentFolder),
        None
    );
    assert_eq!(notes(Scope::CurrentFolder).len(), 2);
}

#[test]
fn a_row_label_lines_up_name_presence_and_state() {
    let status = ClientStatus {
        client: ClientId::Codex,
        presence: Presence::Detected,
        config: None,
        state: ConfigState::DifferentEntry,
    };
    assert_eq!(
        row_label(&status),
        "Codex        detected      ≠ different entry"
    );
}

#[test]
fn server_lines_name_the_server_or_say_how_to_install_it() {
    let found = ServerState::Found(server());
    assert_eq!(
        server_lines(&found),
        [PathMessage::quoting("Server: ", server().as_str(), "")]
    );
    let missing: Vec<String> = server_lines(&ServerState::Missing)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        missing,
        [
            "Server: ffm-mcp was not found next to ffm or on PATH.",
            "Install it from the FFM sources: cargo install --path mcp",
        ]
    );
    let odd_path = absolute("/srv/x/odd");
    let odd = ServerState::NotUnicode(odd_path.clone());
    assert_eq!(
        server_lines(&odd),
        [
            PathMessage::quoting(
                "Server: ",
                odd_path.to_string_lossy(),
                " is not valid Unicode, so no config can name it."
            ),
            PathMessage::from(INSTALL_HINT.to_owned()),
        ]
    );
}

/// The words the review's new states and blocker read as, on a dialog row.
#[test]
fn the_new_states_have_their_own_words() {
    assert_eq!(ConfigState::Disabled.label(), "✓ registered (disabled)");
    assert_eq!(
        ConfigState::Unverified { command: None }.label(),
        "• entry present, server not found"
    );
    assert_eq!(
        ConfigState::ProgramMissing.label(),
        "≠ entry runs a missing program"
    );
    assert_eq!(
        ConfigState::Blocked(Blocker::SameAsUserConfig).label(),
        "✗ same file as All projects"
    );
    assert!(Blocker::SameAsUserConfig
        .explain("/srv/x/.kiro/settings/mcp.json")
        .starts_with("/srv/x/.kiro/settings/mcp.json is also the All projects config"));
    for state in [
        ConfigState::Disabled,
        ConfigState::Unverified { command: None },
        ConfigState::ProgramMissing,
    ] {
        assert!(!state.is_actionable(), "{state:?}");
    }
}
