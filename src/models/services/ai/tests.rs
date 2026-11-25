//! Purpose: Unit tests for `AiDialog` reply formatting without command or token counts.

use super::*;

#[test]
fn apply_reply_handles_response_without_command_or_token_usage() {
    let mut dialog = AiDialog::new(AiSettings::default());
    dialog.apply_reply(Ok(AiResponse {
        provider: AiProvider::Codex,
        model: "gpt-5".into(),
        text: "Plain answer".into(),
        command: None,
        input_tokens: None,
        output_tokens: None,
    }));
    assert!(dialog.last_command.is_none());
    assert_eq!(dialog.status, "Codex [gpt-5]");
    assert_eq!(dialog.messages.len(), 1);
}
