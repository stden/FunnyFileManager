//! Purpose: Replacing an entry: a failed remove or add, an unverified write, quoting and kept keys.

use super::*;

#[test]
fn a_remove_that_fails_stops_before_the_add() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture
        .machine
        .claude()
        .behave(ClaudeBehaviour::Fail("locked".to_owned()));
    let registration = fixture.register();
    let Outcome::Failed(failure) = &registration.outcome else {
        panic!("must fail: {:?}", registration.outcome);
    };
    assert!(matches!(failure, Failure::CliFailed { .. }), "{failure:?}");
    assert_eq!(fixture.machine.claude().calls(), [fixture.remove()]);
    // Review finding: the advice named only the add, which then fails with
    // "already exists". Both steps are still to run.
    let advice = failure.describe("x");
    assert!(
        advice.contains(&format!(
            "Run by hand: claude mcp remove --scope user ffm-mcp && claude mcp add-json --scope user ffm-mcp {}",
            crate::console::shell_word(&fixture.replaced().to_string())
        )),
        "{advice}"
    );
}

#[test]
fn an_add_that_fails_after_the_remove_says_the_entry_is_gone() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture
        .machine
        .claude()
        .behave(ClaudeBehaviour::FailAdd("disk full".to_owned()));
    let Outcome::Failed(failure) = fixture.register().outcome else {
        panic!("must fail");
    };
    let Failure::RemovedNotAdded { message, command } = &failure else {
        panic!("expected RemovedNotAdded, got {failure:?}");
    };
    assert_eq!(message, "disk full (exit 1)");
    assert!(
        command.starts_with("claude mcp add-json --scope user ffm-mcp "),
        "{command}"
    );
    assert!(failure
        .describe("x")
        .starts_with("the old entry was removed"));
}

#[test]
fn an_exit_zero_without_a_write_is_not_verified() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Pretend);
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::NotVerified)
    );
}

#[test]
fn a_server_path_with_spaces_is_quoted_for_the_person_reading_it() {
    let fixture = fixture();
    let call = CliCall {
        args: vec!["mcp".to_owned(), "/srv/x/my tools/ffm-mcp".to_owned()],
        ..fixture.add()
    };
    let quoted = if cfg!(windows) {
        "\"/srv/x/my tools/ffm-mcp\""
    } else {
        "'/srv/x/my tools/ffm-mcp'"
    };
    assert_eq!(command_line(&call), format!("claude mcp {quoted}"));
}

/// Review finding: replacing an entry with `remove` and `add` kept only the
/// command, so the owner's `env` and other keys were lost. `add-json` is
/// handed the old entry with this server set.
#[test]
fn a_replaced_entry_keeps_its_env_and_other_keys() {
    let fixture = fixture();
    let config = fixture.machine.home().join(".claude.json");
    let before = serde_json::json!({"oauthAccount": {"id": "x"}, "mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": "/old/ffm-mcp", "args": ["-v"], "timeout": 5000,
        "env": {"FFM_MCP_TOKEN": "secret", "FFM_MCP_ROOT": "/srv/x/other"}
    }}});
    fs::write(&config, before.to_string()).unwrap();
    assert_eq!(fixture.register().outcome, Outcome::Updated);
    let after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        after["mcpServers"]["ffm-mcp"],
        serde_json::json!({
            "type": "stdio", "command": fixture.desired.command.as_str(), "args": [],
            "timeout": 5000, "env": {"FFM_MCP_TOKEN": "secret"}
        })
    );
    assert_eq!(after["oauthAccount"]["id"], "x");
}

/// Review finding: "not verified" after a remove that worked did not say
/// the old entry was already gone.
#[test]
fn an_unverified_replace_says_the_old_entry_is_gone() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture.machine.claude().behave(ClaudeBehaviour::Pretend);
    let Outcome::Failed(Failure::RemovedNotAdded { message, command }) = fixture.register().outcome
    else {
        panic!("an unverified replace leaves no entry");
    };
    assert!(
        message.starts_with("claude reported success but "),
        "{message}"
    );
    assert!(
        command.starts_with("claude mcp add-json --scope user ffm-mcp "),
        "{command}"
    );
}
