//! Unit tests for [`super`]: where each user config really is (Claude Code's
//! as its CLI decides), a folder whose config is the user-level one, and the
//! files a config read must refuse — review findings, each on a fake machine.

use crate::test_support::FakeMachine;

use super::*;

fn machine() -> FakeMachine {
    FakeMachine::new().expect("fake machine")
}

fn claude_config(host: &Host) -> PathBuf {
    user_config(host, ClientId::ClaudeCode.client()).expect("a home folder")
}

/// Review finding: the panel at the home folder made Current folder name
/// the very files All projects writes, for Codex, Junie and Kiro.
#[test]
fn a_folder_whose_config_is_the_user_level_one_is_blocked() {
    let machine = machine();
    machine.install_server().unwrap();
    let home = ProjectDir::new(machine.home()).unwrap();
    let survey = survey(&machine.host(), Scope::CurrentFolder, Some(home));
    for status in &survey.clients {
        let expected = match status.client {
            // `.mcp.json` is never Claude Code's user config.
            ClientId::ClaudeCode => ConfigState::NoConfig,
            ClientId::Codex | ClientId::Junie | ClientId::Kiro => {
                ConfigState::Blocked(Blocker::SameAsUserConfig)
            }
        };
        assert_eq!(status.state, expected, "{:?}", status.client);
        assert!(status.config.is_some(), "the file is still named");
    }
}

/// The same file under another spelling — a home reached through a link, an
/// override that points into the folder — is still the same file, created
/// yet or not.
#[cfg(unix)]
#[test]
fn another_spelling_of_the_user_config_is_the_same_file() {
    let machine = machine();
    let link = machine.folder().join("home-link");
    std::os::unix::fs::symlink(machine.home(), &link).unwrap();
    let mut host = machine.host();
    host.home = Some(link);
    let home = ProjectDir::new(machine.home()).unwrap();
    let target = Target::CurrentFolder(&home);
    let kiro = ClientId::Kiro.client();
    let path = config_path(&host, kiro, target).unwrap();
    assert_eq!(
        shared_with_all_projects(&host, kiro, target, &path),
        Some(Blocker::SameAsUserConfig)
    );

    let project = ProjectDir::new(machine.folder()).unwrap();
    let inside = machine.folder().join(".codex");
    fs::create_dir(&inside).unwrap();
    host.env.insert("CODEX_HOME", inside.into());
    let codex = ClientId::Codex.client();
    let target = Target::CurrentFolder(&project);
    let path = config_path(&host, codex, target).unwrap();
    assert_eq!(
        shared_with_all_projects(&host, codex, target, &path),
        Some(Blocker::SameAsUserConfig)
    );
}

#[test]
fn a_project_folder_and_all_projects_never_overlap() {
    let machine = machine();
    let host = machine.host();
    let project = ProjectDir::new(machine.folder()).unwrap();
    for client in &CLIENTS {
        for target in [Target::AllProjects, Target::CurrentFolder(&project)] {
            let path = config_path(&host, client, target).unwrap();
            assert_eq!(
                shared_with_all_projects(&host, client, target, &path),
                None,
                "{:?} {target:?}",
                client.id
            );
        }
    }
}

#[test]
fn a_home_that_is_not_a_folder_is_no_home() {
    let machine = machine();
    let mut host = machine.host();
    host.home = Some(machine.home().join("gone"));
    for client in &CLIENTS {
        assert_eq!(
            client_home(&host, client),
            Err(Blocker::NoHomeFolder),
            "{:?}",
            client.id
        );
    }
}

/// Review finding: FFM read `~/.claude.json` where the CLI reads another
/// file — so the row was wrong, and a successful add came back unverified.
#[test]
fn claude_code_user_config_is_where_the_cli_looks() {
    let machine = machine();
    let mut host = machine.host();
    assert_eq!(claude_config(&host), machine.home().join(".claude.json"));

    host.env
        .insert(CLAUDE_CUSTOM_OAUTH_VAR, "https://login.example".into());
    assert_eq!(
        claude_config(&host),
        machine.home().join(".claude-custom-oauth.json")
    );

    let legacy = machine.home().join(".claude").join(".config.json");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "{}").unwrap();
    assert_eq!(claude_config(&host), legacy, "a legacy config wins");
}

#[test]
fn claude_config_dir_moves_both_the_config_and_the_legacy_file() {
    let machine = machine();
    let dir = machine.home().join("claude-dir");
    fs::create_dir(&dir).unwrap();
    let mut host = machine.host();
    host.env.insert("CLAUDE_CONFIG_DIR", dir.clone().into());
    assert_eq!(claude_config(&host), dir.join(".claude.json"));
    // `~/.claude/.config.json` no longer counts: the folder moved.
    fs::create_dir(machine.home().join(".claude")).unwrap();
    fs::write(machine.home().join(".claude").join(".config.json"), "{}").unwrap();
    assert_eq!(claude_config(&host), dir.join(".claude.json"));
    fs::write(dir.join(".config.json"), "{}").unwrap();
    assert_eq!(claude_config(&host), dir.join(".config.json"));
}

/// Review finding: a FIFO named `.mcp.json` in the panel's folder blocked
/// the dialog and `:mcp` for good.
#[cfg(unix)]
#[test]
fn a_fifo_is_refused_without_blocking() {
    let machine = machine();
    let fifo = machine.folder().join(".mcp.json");
    crate::test_support::make_fifo(&fifo).unwrap();
    let timeout = std::time::Duration::from_secs(5);
    let loaded = crate::test_support::run_with_timeout(timeout, move || load(&fifo))
        .expect("reading a writer-less FIFO must not block");
    assert_eq!(
        loaded,
        Err(Blocker::Unreadable("not a regular file".to_owned()))
    );
}

#[test]
fn a_folder_where_the_config_should_be_is_refused() {
    let machine = machine();
    let config = machine.folder().join(".mcp.json");
    fs::create_dir(&config).unwrap();
    assert_eq!(
        load(&config),
        Err(Blocker::Unreadable("not a regular file".to_owned()))
    );
}

/// Review finding: a link to `/dev/zero` would have been read until memory
/// ran out; a config past the limit is refused unread.
#[test]
fn a_config_past_the_size_limit_is_refused() {
    let machine = machine();
    let config = machine.folder().join(".mcp.json");
    let file = fs::File::create(&config).unwrap();
    file.set_len(MAX_CONFIG_BYTES + 1).unwrap();
    let Err(Blocker::Unreadable(message)) = load(&config) else {
        panic!("a sparse file past the limit must be refused");
    };
    assert!(message.contains("more than"), "{message}");
}

/// Any other failure to read is named, not taken for "no config".
#[cfg(unix)]
#[test]
fn a_link_that_loops_is_unreadable() {
    let machine = machine();
    let config = machine.folder().join(".mcp.json");
    std::os::unix::fs::symlink(&config, &config).unwrap();
    assert!(matches!(load(&config), Err(Blocker::Unreadable(_))));
}
