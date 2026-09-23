//! Purpose: Verify the language-server inventory is a discoverable built-in command.

#[test]
fn lsp_is_a_builtin_and_is_offered_by_completion() {
    assert_eq!(format!("{:?}", super::Command::parse("lsp")), "Lsp");
    assert!(super::VERBS.contains(&"lsp"));
}
