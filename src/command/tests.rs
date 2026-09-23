//! Purpose: Unit tests for the `:` command-line parser.
//!
use super::*;
use crate::models::SortMode;

mod tests_tilde;

#[test]
fn stats_opens_the_action_timing_report() {
    assert!(!matches!(Command::parse("stats"), Command::Shell(_)));
}

#[test]
fn test_command_parse_basic() {
    assert_eq!(Command::parse("pwd"), Command::Pwd);
    assert_eq!(Command::parse("help"), Command::Help);
    assert_eq!(Command::parse("quit"), Command::Quit);
    assert_eq!(Command::parse("exit"), Command::Quit);
    assert_eq!(Command::parse("mcp"), Command::Mcp);
    assert_eq!(Command::parse("mark all"), Command::MarkAll);
    assert_eq!(Command::parse("unmark"), Command::UnmarkAll);
}

#[test]
fn bare_wsl_opens_the_manager_and_is_offered_for_completion() {
    for input in ["wsl", "  wsl  ", "\twsl\n"] {
        assert_eq!(Command::parse(input), Command::Wsl);
    }
    assert!(VERBS.contains(&"wsl"));
}

#[test]
fn wsl_with_arguments_preserves_the_shell_request() {
    for input in [
        "wsl --list --verbose",
        "wsl -d Ubuntu",
        "  wsl --update  ",
        "wsl --exec sh -c 'printf hello'",
        "wsl.exe",
        "WSL",
    ] {
        assert_eq!(
            Command::parse(input),
            Command::Shell(ShellCommand::new(input).expect("non-empty shell command")),
        );
    }
}

#[cfg(windows)]
#[test]
fn codex_launches_outside_the_captured_shell() {
    for input in [
        "codex",
        "  codex --version  ",
        "CODEX\tresume --last",
        r#"codex --model example -C "My Project" "Explain this code""#,
        "codex.exe --help",
        "codex.cmd resume",
    ] {
        assert_eq!(
            Command::parse(input),
            Command::NewConsole(ShellCommand::new(input).expect("interactive command"))
        );
    }
}

#[test]
fn codex_mentions_and_other_programs_keep_normal_shell_execution() {
    for input in ["echo codex", "codex-auto", "codex-other --help", "mycodex"] {
        assert_eq!(
            Command::parse(input),
            Command::Shell(ShellCommand::new(input).expect("shell command"))
        );
    }
}

#[cfg(not(windows))]
#[test]
fn codex_keeps_normal_shell_execution_on_unix() {
    let input = "codex --help";
    assert_eq!(
        Command::parse(input),
        Command::Shell(ShellCommand::new(input).expect("shell command"))
    );
}

#[test]
fn test_command_parse_sort() {
    assert_eq!(Command::parse("sort name"), Command::Sort(SortMode::Name));
    assert_eq!(Command::parse("sort"), Command::Sort(SortMode::Name)); // bare sort
    assert_eq!(
        Command::parse("sort ext"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(
        Command::parse("sort extension"),
        Command::Sort(SortMode::Extension)
    );
    assert_eq!(Command::parse("sort size"), Command::Sort(SortMode::Size));
    assert_eq!(
        Command::parse("sort blah"),
        Command::Invalid {
            input: "sort blah".into(),
            usage: SORT_USAGE
        }
    );
}

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

// ─── verb table ───────────────────────────────────────────────────────

#[test]
fn every_completion_verb_parses_to_a_real_command() {
    // Verbs that need an argument get a minimal valid one; the point is that a
    // verb the completer offers is never dispatched as a shell command.
    for verb in VERBS {
        let line = match *verb {
            "cd" => "cd /".to_string(),
            "mark" => "mark all".to_string(),
            "bookmark" => "bookmark go 0".to_string(),
            other => other.to_string(),
        };
        assert!(
            !matches!(
                Command::parse(&line),
                Command::Shell(_) | Command::Invalid { .. }
            ),
            "verb `{verb}` is offered for completion but does not parse"
        );
    }
}

#[test]
fn shell_command_rejects_blank_text() {
    // The constructor is the only way in, so `Command::Shell` cannot carry an
    // empty command and the dispatcher needs no runtime check for one.
    assert!(ShellCommand::new("").is_none());
    assert!(ShellCommand::new("   \t ").is_none());
    assert_eq!(
        ShellCommand::new("  ls -l  ").map(|c| c.as_str().to_owned()),
        Some("  ls -l  ".to_owned()),
        "surrounding space is preserved — only all-blank input is rejected"
    );
}

#[test]
fn shell_command_displays_as_its_text() {
    let command = ShellCommand::new("grep -r needle .").expect("non-empty");
    assert_eq!(command.to_string(), "grep -r needle .");
}

/// `:sessions` opens the panel; `:sessions import netbox [path]` imports,
/// the path being the rest of the line.
#[test]
fn sessions_parses_its_verb_the_import_and_a_path_with_spaces() {
    use super::SessionsCommand;
    assert_eq!(
        Command::parse("sessions"),
        Command::Sessions(SessionsCommand::Open)
    );
    assert_eq!(
        Command::parse("sessions import netbox"),
        Command::Sessions(SessionsCommand::ImportNetBox(None))
    );
    assert_eq!(
        Command::parse("  sessions   import  netbox  \"/srv/x/my host.netbox\"  "),
        Command::Sessions(SessionsCommand::ImportNetBox(Some(PathBuf::from(
            "/srv/x/my host.netbox"
        ))))
    );
    assert!(VERBS.contains(&"sessions"));
}

/// Anything else after `sessions` is the usage.
#[test]
fn sessions_with_other_words_is_its_usage() {
    for bad in [
        "sessions import",
        "sessions import far",
        "sessions import NETBOX x",
        "sessions open",
    ] {
        assert!(
            matches!(
                Command::parse(bad),
                Command::Invalid { usage, .. }
                    if usage == "sessions expects nothing, or import netbox [path]"
            ),
            "{bad}"
        );
    }
}
