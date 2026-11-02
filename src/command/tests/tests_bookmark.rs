//! Purpose: `bookmark set|go <0-9>` and the usage for anything else.
use super::*;

#[test]
fn test_command_parse_bookmark() {
    assert_eq!(
        Command::parse("bookmark set 3"),
        Command::BookmarkSet(BookmarkSlot::new(3).unwrap())
    );
    assert_eq!(
        Command::parse("bookmark go 0"),
        Command::BookmarkGo(BookmarkSlot::new(0).unwrap())
    );
    assert_eq!(
        Command::parse("bookmark set 99"),
        Command::Invalid {
            input: "bookmark set 99".into(),
            usage: BOOKMARK_USAGE
        }
    );
    assert_eq!(
        Command::parse("bookmark go abc"),
        Command::Invalid {
            input: "bookmark go abc".into(),
            usage: BOOKMARK_USAGE
        }
    );
    assert_eq!(
        Command::parse("bookmark foo"),
        Command::Invalid {
            input: "bookmark foo".into(),
            usage: BOOKMARK_USAGE
        }
    );
}
