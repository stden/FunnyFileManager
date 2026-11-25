//! Purpose: Typed arguments and parsers for `:jira`, `:mcps`, `:ai`, and `:ask`.

use super::Command;
use crate::values::ai::AiProvider;

/// What `:jira` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JiraCommand {
    Browse,
    Jql(String),
    Export(Option<String>),
    Snapshot(Option<String>),
    Edit,
}

/// What `:mcps` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpServicesCommand {
    Browse,
    Edit,
}

/// What `:ai` / `:ask` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiCommand {
    Open(Option<String>),
    Provider(AiProvider),
    Edit,
}

const JIRA_USAGE: &str = "jira expects [edit | jql <query> | export [KEY] | snapshot [query]]";
const MCPS_USAGE: &str = "mcps expects [edit]";
const AI_USAGE: &str = "ai expects [edit | anthropic|openai|codex | <prompt>]";

pub(super) fn parse_jira(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Command {
    match (parts.next(), rest_tail(input, 2)) {
        (None, _) => Command::Jira(JiraCommand::Browse),
        (Some("edit"), None) => Command::Jira(JiraCommand::Edit),
        (Some("jql"), Some(q)) => Command::Jira(JiraCommand::Jql(q)),
        (Some("export"), key)
            if key
                .as_ref()
                .is_none_or(|k| !k.contains(char::is_whitespace)) =>
        {
            Command::Jira(JiraCommand::Export(key))
        }
        (Some("snapshot"), jql) => Command::Jira(JiraCommand::Snapshot(jql)),
        (Some(_), _) => Command::invalid(input, JIRA_USAGE),
    }
}

pub(super) fn parse_mcps(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Command {
    match (parts.next(), parts.next()) {
        (None, _) => Command::McpServices(McpServicesCommand::Browse),
        (Some("edit"), None) => Command::McpServices(McpServicesCommand::Edit),
        _ => Command::invalid(input, MCPS_USAGE),
    }
}

pub(super) fn parse_ai(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Command {
    let first = parts.next();
    let second = parts.next();
    if first == Some("edit") && second.is_none() {
        return Command::Ai(AiCommand::Edit);
    }
    if first == Some("edit") {
        return Command::invalid(input, AI_USAGE);
    }
    if let (Some(name), None) = (first, second) {
        if let Some(provider) = AiProvider::parse(name) {
            return Command::Ai(AiCommand::Provider(provider));
        }
    }
    Command::Ai(AiCommand::Open(rest_tail(input, 1)))
}

fn rest_tail(input: &str, skip_words: usize) -> Option<String> {
    let mut rest = input.trim_start();
    for _ in 0..skip_words {
        let (_, tail) = rest.split_once(char::is_whitespace)?;
        rest = tail.trim_start();
    }
    let trimmed = rest.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests;
