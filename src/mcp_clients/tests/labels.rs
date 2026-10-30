//! Purpose: The words a config state and a blocker read as on a dialog row,
//! which of them are actionable, and how a blocker explains itself.

use super::*;

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
