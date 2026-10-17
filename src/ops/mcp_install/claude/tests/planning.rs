//! Purpose: What a registration runs: an add, a remove then add, nothing, or nothing at all when blocked.

use super::*;

#[test]
fn an_absent_entry_is_added_with_one_call_in_the_home_folder() {
    let fixture = fixture();
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(fixture.machine.claude().calls(), [fixture.add()]);
    // Quoted as the platform's shell needs it: a Windows path has
    // backslashes, and the runner's temp folder a `~`.
    assert_eq!(
        registration.ran,
        [format!(
            "claude mcp add --scope user --transport stdio ffm-mcp -- {}",
            crate::console::shell_word(fixture.desired.command.as_str())
        )]
    );
    assert_eq!(
        registration.config,
        Some(fixture.machine.home().join(".claude.json"))
    );
}

#[test]
fn a_different_entry_is_removed_then_added() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Updated);
    assert_eq!(
        fixture.machine.claude().calls(),
        [fixture.remove(), fixture.add_json(&fixture.replaced())]
    );
    assert_eq!(registration.ran.len(), 2);
    let text = fs::read_to_string(fixture.machine.home().join(".claude.json")).unwrap();
    assert!(
        text.contains("oauthAccount"),
        "the sign-in survives: {text}"
    );
}

#[test]
fn a_registered_entry_runs_nothing() {
    let fixture = fixture();
    fixture.write_entry(fixture.desired.command.as_str());
    assert_eq!(fixture.register().outcome, Outcome::Unchanged);
    assert!(fixture.machine.claude().calls().is_empty());
}

#[test]
fn a_config_that_does_not_parse_is_never_handed_to_the_cli() {
    let fixture = fixture();
    fs::write(fixture.machine.home().join(".claude.json"), "{ broken").unwrap();
    let registration = fixture.register();
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unparsable(_)))
    ));
    assert!(fixture.machine.claude().calls().is_empty());
}

#[test]
fn no_home_folder_stops_before_the_cli() {
    let mut fixture = fixture();
    fixture.host.home = None;
    let registration = fixture.register();
    assert_eq!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::NoHomeFolder))
    );
    assert_eq!(registration.config, None);
}
