//! Purpose: Which config the CLI changes and checks, and homes and configs that stop it.

use super::*;

/// Review finding: `NotFound` from a missing working or temp folder was
/// reported as "claude CLI not found" while `claude` sat on PATH.
#[test]
fn not_found_while_the_program_is_there_is_a_cli_failure() {
    let fixture = fixture();
    let mut host = fixture.host.clone();
    host.runner = std::sync::Arc::new(Refuses(io::ErrorKind::NotFound));
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) =
        register(&host, &fixture.desired).outcome
    else {
        panic!("must be a CLI failure, not a missing CLI");
    };
    assert_eq!(exit, None);
    assert!(message.contains("temporary file"), "{message}");
}

#[test]
fn a_home_folder_that_is_gone_stops_before_the_cli() {
    let mut fixture = fixture();
    fixture.host.home = Some(fixture.machine.home().join("gone"));
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::Blocked(Blocker::NoHomeFolder))
    );
    assert_eq!(fixture.machine.claude().calls().len(), 0);
}

/// Review finding: under `CLAUDE_CONFIG_DIR` the CLI writes another file
/// than the one FFM read and checked; no test followed the CLI there.
#[test]
fn claude_config_dir_is_where_the_entry_is_added_and_checked() {
    let mut fixture = fixture();
    let dir = fixture.machine.home().join("claude-dir");
    fs::create_dir(&dir).unwrap();
    fixture
        .host
        .env
        .insert("CLAUDE_CONFIG_DIR", dir.clone().into());
    fixture
        .machine
        .claude()
        .use_config(dir.join(".claude.json"));
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(registration.config, Some(dir.join(".claude.json")));
    assert_eq!(fixture.machine.claude().calls()[0].cwd, dir);
    assert!(!fixture.machine.home().join(".claude.json").exists());
}

#[test]
fn a_legacy_config_is_where_the_entry_is_added_and_checked() {
    let fixture = fixture();
    let legacy = fixture.machine.home().join(".claude").join(".config.json");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "{}").unwrap();
    fixture.machine.claude().use_config(legacy.clone());
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(registration.config, Some(legacy));
}

#[test]
fn a_user_config_that_cannot_be_read_is_never_handed_to_the_cli() {
    let fixture = fixture();
    fs::create_dir(fixture.machine.home().join(".claude.json")).unwrap();
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unreadable(
            "not a regular file".to_owned()
        )))
    );
    assert_eq!(fixture.machine.claude().calls().len(), 0);
}
