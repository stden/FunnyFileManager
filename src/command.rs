//! Purpose: Parses colon commands into typed command values used by handlers.
//!
use std::path::{Path, PathBuf};

use crate::models::{BookmarkSlot, SortMode};

/// Every verb [`Command::parse`] accepts. Single source of truth: the
/// tab-completer in [`crate::complete`] reads this table instead of keeping
/// its own, so a verb can never be accepted but not offered.
pub const VERBS: &[&str] = &[
    "cd",
    "pwd",
    "sort",
    "bookmark",
    "mark",
    "unmark",
    "help",
    "stats",
    "winmsg",
    "winmsgs",
    "winerr",
    "winerrors",
    "winperf",
    "wincheck",
    "wsl",
    "mcp",
    "lsp",
    "cleanup",
    "quit",
    "exit",
];

/// Type-safe representation of all supported command-line commands.
/// Using an enum instead of raw strings gives us exhaustiveness checking,
/// easier testing, and prevents typos in command dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// cd <path>
    Cd(PathBuf),
    /// pwd
    Pwd,
    /// sort name|ext|size
    Sort(SortMode),
    /// bookmark set|go <0-9>
    BookmarkSet(BookmarkSlot),
    BookmarkGo(BookmarkSlot),
    /// mark all
    MarkAll,
    /// unmark
    UnmarkAll,
    /// help
    Help,
    /// Show time spent processing and rendering user actions.
    Stats,
    /// winmsg / winerrors / wincheck etc.
    WinMsg,
    WinPerf,
    /// Open the WSL manager. Arguments still select the platform shell command.
    Wsl,
    /// mcp
    Mcp,
    /// Show installed language servers and launch parameters.
    Lsp,
    /// cleanup — open the disk cleanup dialog.
    DiskCleanup,
    /// quit / exit
    Quit,
    /// Any other input is treated as a shell command.
    Shell(ShellCommand),
    /// An interactive command launched in a separate Windows console.
    NewConsole(ShellCommand),
    /// Nothing was typed. Its own variant rather than an empty `Shell`, so the
    /// dispatcher's "there is a command to run" case cannot be reached without
    /// one — see [`ShellCommand`].
    Empty,
    /// Known verb with bad arguments (e.g. "bookmark set 99", "sort blah").
    /// `usage` carries the correct form so the status bar can tell the user
    /// what to type instead of a generic "invalid syntax".
    Invalid {
        input: String,
        usage: &'static str,
    },
}

/// A command line proven to hold something a shell could run.
///
/// The only way to obtain one is [`ShellCommand::new`], which rejects
/// whitespace-only input. `Command::Shell` therefore cannot carry an empty
/// command, and the dispatcher does not need a runtime check for one — the
/// same construction [`DurableLine`](crate::history::DurableLine) uses to make
/// "this text is safe to persist" a property of the type rather than of a
/// forgotten `if`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand(String);

impl ShellCommand {
    /// `Some` iff `text` holds a non-whitespace character.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        (!text.trim().is_empty()).then(|| Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ShellCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Correct forms for the verbs that take arguments. These are the single
/// source of truth for both the parse failure and the message the user reads,
/// so a syntax change can never leave the status bar advertising the old one.
const CD_USAGE: &str = "cd requires a path";
const SORT_USAGE: &str = SortMode::USAGE;
const BOOKMARK_USAGE: &str = "bookmark expects set|go <0-9>";
const MARK_USAGE: &str = "mark expects all";

impl Command {
    /// A known verb used with arguments it does not accept.
    fn invalid(input: &str, usage: &'static str) -> Self {
        Self::Invalid {
            input: input.to_string(),
            usage,
        }
    }

    /// Parse a raw command line string into a typed Command.
    /// This is the single source of truth for command syntax.
    pub fn parse(input: &str) -> Self {
        let mut parts = input.split_whitespace();
        let Some(cmd) = parts.next() else {
            return Self::Empty;
        };

        match cmd {
            "cd" => cd_argument(input).map_or_else(|| Self::invalid(input, CD_USAGE), Self::Cd),
            "pwd" => Self::Pwd,
            // A bare `sort` keeps its old meaning: back to name order.
            "sort" => parts
                .next()
                .map_or(Some(SortMode::Name), SortMode::parse)
                .map_or_else(|| Self::invalid(input, SORT_USAGE), Self::Sort),
            "bookmark" => Self::parse_bookmark(input, &mut parts),
            "mark" => Self::parse_mark(input, parts.next()),
            "unmark" => Self::UnmarkAll,
            "help" => Self::Help,
            "stats" => Self::Stats,
            "winmsg" | "winmsgs" | "winerr" | "winerrors" => Self::WinMsg,
            "winperf" | "wincheck" => Self::WinPerf,
            "wsl" if parts.next().is_none() => Self::Wsl,
            "mcp" => Self::Mcp,
            "lsp" => Self::Lsp,
            "cleanup" => Self::DiskCleanup,
            "quit" | "exit" => Self::Quit,
            // `input` has a non-whitespace token — `parts.next()` just yielded
            // one — so the constructor cannot fail; `Empty` above owns the
            // other case.
            _ => Self::parse_external(input, cmd),
        }
    }

    fn parse_external(input: &str, verb: &str) -> Self {
        let Some(command) = ShellCommand::new(input) else {
            return Self::Empty;
        };
        if cfg!(windows)
            && ["codex", "codex.exe", "codex.cmd"]
                .iter()
                .any(|name| verb.eq_ignore_ascii_case(name))
        {
            Self::NewConsole(command)
        } else {
            Self::Shell(command)
        }
    }

    fn parse_mark(input: &str, argument: Option<&str>) -> Self {
        if argument == Some("all") {
            Self::MarkAll
        } else {
            Self::invalid(input, MARK_USAGE)
        }
    }

    fn parse_bookmark(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Self {
        match (parts.next(), parts.next()) {
            // `set` and `go` share one slot check — the only difference is
            // which variant a valid slot produces.
            (Some(verb @ ("set" | "go")), Some(slot_str)) => slot_str
                .parse::<usize>()
                .ok()
                .and_then(|n| BookmarkSlot::from_usize(n).ok())
                .map_or_else(
                    || Self::invalid(input, BOOKMARK_USAGE),
                    |slot| {
                        if verb == "set" {
                            Self::BookmarkSet(slot)
                        } else {
                            Self::BookmarkGo(slot)
                        }
                    },
                ),
            _ => Self::invalid(input, BOOKMARK_USAGE),
        }
    }
}

/// The `cd` argument is everything after the verb, not the first whitespace
/// token: there is no quoting engine here, so taking one token would make a
/// directory whose name contains a space unreachable — and the tab-completer
/// happily produces such lines. One matching pair of surrounding quotes is
/// stripped so `cd "My Documents"` behaves the way a shell taught the user.
fn cd_argument(input: &str) -> Option<PathBuf> {
    let (_, rest) = input.trim_start().split_once(char::is_whitespace)?;
    let arg = strip_quotes(rest.trim());
    (!arg.is_empty()).then(|| PathBuf::from(arg))
}

/// Remove one matching pair of surrounding `"` or `'` quotes, if present.
fn strip_quotes(arg: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| arg.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(arg)
}

/// Expand a leading `~` (`~` alone or `~/…`) against the user's home directory.
///
/// Both `cd` (see `handlers::dispatch`) and path completion (see
/// [`crate::complete`]) resolve through here, so the directory the completer
/// offers is the directory `cd` then opens.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| path.to_path_buf(), |home| PathBuf::from(home).join(rest))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_lsp;
