//! Purpose: The `cd` argument — the whole rest of the line, one pair of quotes
//! stripped — and the usage when there is none.
use super::*;

#[test]
fn test_command_parse_cd_and_shell() {
    assert_eq!(
        Command::parse("cd /tmp"),
        Command::Cd(PathBuf::from("/tmp"))
    );
    assert_eq!(
        Command::parse("cd"),
        Command::Invalid {
            input: "cd".into(),
            usage: CD_USAGE
        }
    );
    assert_eq!(
        Command::parse("ls -l"),
        Command::Shell(ShellCommand::new("ls -l").expect("non-empty"))
    );
    // An empty line is its own variant: `Command::Shell` cannot represent it.
    assert_eq!(Command::parse(""), Command::Empty);
    assert_eq!(Command::parse("   "), Command::Empty);
}

// ─── cd argument: the whole rest of the line ──────────────────────────

#[test]
fn cd_takes_the_whole_rest_of_the_line() {
    assert_eq!(
        Command::parse("cd My Documents"),
        Command::Cd(PathBuf::from("My Documents"))
    );
    assert_eq!(
        Command::parse("cd  /tmp/a b c "),
        Command::Cd(PathBuf::from("/tmp/a b c"))
    );
}

#[test]
fn cd_strips_one_pair_of_surrounding_quotes() {
    assert_eq!(
        Command::parse(r#"cd "My Documents""#),
        Command::Cd(PathBuf::from("My Documents"))
    );
    assert_eq!(
        Command::parse("cd 'My Documents'"),
        Command::Cd(PathBuf::from("My Documents"))
    );
    // Unbalanced quotes are part of the name, not syntax.
    assert_eq!(
        Command::parse(r#"cd "half"#),
        Command::Cd(PathBuf::from("\"half"))
    );
}

#[test]
fn cd_without_an_argument_is_invalid() {
    assert_eq!(
        Command::parse("cd   "),
        Command::Invalid {
            input: "cd   ".into(),
            usage: CD_USAGE
        }
    );
    assert_eq!(
        Command::parse("cd \"\""),
        Command::Invalid {
            input: "cd \"\"".into(),
            usage: CD_USAGE
        }
    );
}
