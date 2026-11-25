//! Purpose: Parser tests for `:jira`, `:mcps`, `:ai`, and `:ask` colon commands.

use super::*;
use crate::values::ai::AiProvider;

#[test]
fn jira_commands_parse_and_validate() {
    use super::JiraCommand;

    assert_eq!(Command::parse("jira"), Command::Jira(JiraCommand::Browse));
    assert_eq!(
        Command::parse("jira edit"),
        Command::Jira(JiraCommand::Edit)
    );
    assert_eq!(
        Command::parse("jira jql project = DEMO"),
        Command::Jira(JiraCommand::Jql("project = DEMO".into()))
    );
    assert_eq!(
        Command::parse("jira export DEMO-1"),
        Command::Jira(JiraCommand::Export(Some("DEMO-1".into())))
    );
    assert_eq!(
        Command::parse("jira snapshot"),
        Command::Jira(JiraCommand::Snapshot(None))
    );
    assert!(matches!(
        Command::parse("jira unknown arg"),
        Command::Invalid { .. }
    ));
}

#[test]
fn mcps_commands_parse_and_validate() {
    use super::McpServicesCommand;

    assert_eq!(
        Command::parse("mcps"),
        Command::McpServices(McpServicesCommand::Browse)
    );
    assert_eq!(
        Command::parse("mcps edit"),
        Command::McpServices(McpServicesCommand::Edit)
    );
    assert!(matches!(
        Command::parse("mcps bad"),
        Command::Invalid { .. }
    ));
}

#[test]
fn ai_and_ask_commands_parse_and_validate() {
    use super::AiCommand;

    assert_eq!(Command::parse("ai"), Command::Ai(AiCommand::Open(None)));
    assert_eq!(Command::parse("ask"), Command::Ai(AiCommand::Open(None)));
    assert_eq!(Command::parse("ai edit"), Command::Ai(AiCommand::Edit));
    assert!(matches!(
        Command::parse("ai edit x"),
        Command::Invalid { .. }
    ));
    assert_eq!(
        Command::parse("ai codex"),
        Command::Ai(AiCommand::Provider(AiProvider::Codex))
    );
    assert_eq!(
        Command::parse("ask explain rust tests"),
        Command::Ai(AiCommand::Open(Some("explain rust tests".into())))
    );
}
