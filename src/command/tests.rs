//! Purpose: Unit tests for the `:` command-line parser.
//!
use super::*;
use crate::models::SortMode;
use std::env;
use std::ffi::OsString;
use std::sync::MutexGuard;

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

// ─── tilde expansion ──────────────────────────────────────────────────

#[test]
fn expand_tilde_leaves_other_paths_alone() {
    assert_eq!(expand_tilde(Path::new("/abs/x")), PathBuf::from("/abs/x"));
    assert_eq!(expand_tilde(Path::new("rel/x")), PathBuf::from("rel/x"));
    // `~name` is a user reference we do not support — keep it verbatim.
    assert_eq!(expand_tilde(Path::new("~other")), PathBuf::from("~other"));
}

/// RAII swap of the two home-directory variables `expand_tilde` reads.
///
/// Both are removed on construction and restored on drop, under the
/// process-wide env lock — every test in this crate that expands `~` takes the
/// same lock, so a missing `$HOME` can never leak into a parallel test.
struct HomeVars {
    _lock: MutexGuard<'static, ()>,
    home: Option<OsString>,
    userprofile: Option<OsString>,
}

impl HomeVars {
    fn cleared() -> Self {
        let lock = crate::test_support::env_lock();
        let taken = Self {
            _lock: lock,
            home: env::var_os("HOME"),
            userprofile: env::var_os("USERPROFILE"),
        };
        env::remove_var("HOME");
        env::remove_var("USERPROFILE");
        taken
    }
}

impl Drop for HomeVars {
    fn drop(&mut self) {
        restore_var("HOME", self.home.take());
        restore_var("USERPROFILE", self.userprofile.take());
    }
}

fn restore_var(name: &str, value: Option<OsString>) {
    match value {
        Some(value) => env::set_var(name, value),
        None => env::remove_var(name),
    }
}

/// With no home directory to expand against, `~` must survive verbatim. Turning
/// it into a bare relative `marker` would make `cd ~/marker` silently open a
/// subdirectory of the current panel instead.
#[test]
fn expand_tilde_without_any_home_variable_returns_the_path_unchanged() {
    let _guard = HomeVars::cleared();

    assert_eq!(
        expand_tilde(Path::new("~/marker")),
        PathBuf::from("~/marker")
    );
    assert_eq!(expand_tilde(Path::new("~")), PathBuf::from("~"));
}

/// Windows sets `%USERPROFILE%`, not `$HOME`; the fallback is what makes `cd ~`
/// work there, and it is only reachable while `$HOME` is absent.
#[test]
fn expand_tilde_falls_back_to_userprofile_when_home_is_unset() {
    let _guard = HomeVars::cleared();
    env::set_var("USERPROFILE", Path::new("/profiles/dana"));

    assert_eq!(
        expand_tilde(Path::new("~/marker")),
        PathBuf::from("/profiles/dana/marker")
    );
}

#[test]
fn expand_tilde_replaces_the_leading_tilde_component() {
    // Asserted structurally: other tests swap $HOME concurrently, so the exact
    // value is not stable, but an expanded path is always absolute and keeps
    // the rest of the components. The env lock keeps this off the window in
    // which the tests above run without a home directory at all.
    let _lock = crate::test_support::env_lock();
    let expanded = expand_tilde(Path::new("~/marker"));
    assert!(
        expanded.is_absolute(),
        "~ must expand to an absolute path, got {}",
        expanded.display()
    );
    assert!(expanded.ends_with("marker"));
    assert!(!expanded.starts_with("~"));
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
