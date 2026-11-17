//! Purpose: `Command::parse` on built-ins, sort modes, bookmarks, `cd` and blank input.

use super::*;

// --- Tests for the new Command enum parser (the main value of this refactor) ---

#[test]
fn test_command_parse_basic_commands() {
    assert_eq!(Command::parse("pwd"), Command::Pwd);
    assert_eq!(Command::parse("help"), Command::Help);
    assert_eq!(Command::parse("quit"), Command::Quit);
    assert_eq!(Command::parse("exit"), Command::Quit);
    assert_eq!(Command::parse("mcp"), Command::Mcp);
    assert_eq!(Command::parse("mark all"), Command::MarkAll);
    assert_eq!(Command::parse("unmark"), Command::UnmarkAll);
}

#[test]
fn test_command_parse_sort_variants() {
    assert_eq!(Command::parse("sort name"), Command::Sort(SortMode::Name));
    assert_eq!(
        Command::parse("sort ext"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(
        Command::parse("sort extension"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(Command::parse("sort size"), Command::Sort(SortMode::Size));
    // Bad subcommand for known verb now produces Invalid (better domain modeling)
    assert!(matches!(
        Command::parse("sort foo"),
        Command::Invalid { .. }
    ));
}

#[test]
fn test_command_parse_bookmark() {
    assert_eq!(
        Command::parse("bookmark set 4"),
        Command::BookmarkSet(BookmarkSlot::new(4).unwrap())
    );
    assert_eq!(
        Command::parse("bookmark go 7"),
        Command::BookmarkGo(BookmarkSlot::new(7).unwrap())
    );
    assert!(matches!(
        Command::parse("bookmark set 99"),
        Command::Invalid { .. }
    ));
}

#[test]
fn test_command_parse_cd_and_shell_fallback() {
    assert!(matches!(Command::parse("cd /tmp"), Command::Cd(_)));
    assert!(matches!(Command::parse("ls -la"), Command::Shell(_)));
    assert!(matches!(
        Command::parse("some completely unknown thing"),
        Command::Shell(_)
    ));
}

#[test]
fn test_command_parse_empty() {
    // An empty line is `Command::Empty`; `Shell` cannot represent it.
    assert!(matches!(Command::parse(""), Command::Empty));
    assert!(matches!(Command::parse("   "), Command::Empty));
}
