//! Unit tests for `:ai` / `:ask` single-word non-provider prompts.

use super::*;

#[test]
fn single_non_provider_word_opens_ai_with_prompt() {
    assert_eq!(
        Command::parse("ai summarize"),
        Command::Ai(AiCommand::Open(Some("summarize".to_owned())))
    );
    assert_eq!(
        Command::parse("ask status"),
        Command::Ai(AiCommand::Open(Some("status".to_owned())))
    );
}
