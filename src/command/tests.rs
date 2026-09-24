//! Purpose: Unit tests for the `:` command-line parser: the plain verbs, the
//! shell fallback and the verb table. `cd` is pinned in `tests/tests_cd.rs`,
//! `bookmark` in `tests/tests_bookmark.rs`, `sessions` in
//! `tests/tests_sessions.rs` and `~` in `tests/tests_tilde.rs`.
//!
use super::*;

mod tests_bookmark;
mod tests_cd;
mod tests_sessions;
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

// ─── verb table ───────────────────────────────────────────────────────

#[test]
fn every_completion_verb_parses_to_a_real_command() {
    // Verbs that need an argument get a minimal valid one; the point is that a
    // verb the completer offers is never dispatched as a shell command.
    for verb in VERBS {
        let line = match *verb {
            "cd" => "cd /".to_owned(),
            "mark" => "mark all".to_owned(),
            "bookmark" => "bookmark go 0".to_owned(),
            other => other.to_owned(),
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
