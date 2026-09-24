//! Purpose: Verify the Language servers dialog's `lsp` command is a discoverable built-in.

#[test]
fn lsp_is_a_builtin_and_is_offered_by_completion() {
    assert_eq!(super::Command::parse("lsp"), super::Command::Lsp);
    assert!(super::VERBS.contains(&"lsp"));
}

#[test]
fn lsp_commands_are_safe_to_recall_from_durable_history() {
    let line = crate::history::DurableLine::classify(" lsp ").unwrap();
    assert_eq!(line.as_str(), "lsp");
}
