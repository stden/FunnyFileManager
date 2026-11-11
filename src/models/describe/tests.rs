//! Purpose: Verify descriptions can be edited at Unicode boundaries without changing the queue.

use super::DescribePrompt;

#[test]
fn editing_a_description_moves_by_characters_and_preserves_the_pending_files() {
    let mut prompt = DescribePrompt::new("first.txt".into(), "a😀z", vec!["next.txt".into()]);
    prompt.home();
    prompt.right();
    assert_eq!(prompt.after_caret(), "😀z");
    prompt.delete_forward();
    assert_eq!(prompt.text(), "az");
    prompt.push('é');
    assert_eq!(prompt.text(), "aéz");
    prompt.left();
    assert_eq!(prompt.after_caret(), "éz");
    prompt.end();
    prompt.backspace();
    assert_eq!(prompt.text(), "aé");
    assert_eq!(prompt.name(), "first.txt");
    assert_eq!(prompt.pending(), ["next.txt"]);
}

#[test]
fn description_button_focus_does_not_change_the_text_or_caret() {
    let mut prompt = DescribePrompt::new("file.txt".into(), "description", Vec::new());
    prompt.left();
    let line = prompt.line().clone();
    prompt.focus_cancel(true);
    assert!(prompt.on_cancel());
    prompt.toggle_button();
    assert!(!prompt.on_cancel());
    prompt.focus_cancel(false);
    assert_eq!(prompt.line(), &line);
}
