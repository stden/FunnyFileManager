//! Purpose: Parses colon commands into typed command values used by handlers.
//!
use std::path::{Path, PathBuf};

use crate::models::{BookmarkSlot, SortMode};

mod parse;

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
    "sessions",
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
    /// Open the Language servers dialog: inspect and edit the servers.
    Lsp,
    /// cleanup — open the disk cleanup dialog.
    DiskCleanup,
    /// `sessions …`: `NetBox`'s stored sessions.
    Sessions(SessionsCommand),
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

/// What `sessions` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionsCommand {
    /// `sessions`: show the stored-sessions panel.
    Open,
    /// `sessions import netbox [path]`: import a `.netbox` file. The path is
    /// the rest of the line, so a path with spaces needs no quoting; without
    /// one, the `.netbox` file under a file panel's cursor.
    ImportNetBox(Option<PathBuf>),
}

/// Correct forms for the verbs that take arguments. These are the single
/// source of truth for both the parse failure and the message the user reads,
/// so a syntax change can never leave the status bar advertising the old one.
const CD_USAGE: &str = "cd requires a path";
const SORT_USAGE: &str = SortMode::USAGE;
const BOOKMARK_USAGE: &str = "bookmark expects set|go <0-9>";
const MARK_USAGE: &str = "mark expects all";
const SESSIONS_USAGE: &str = "sessions expects nothing, or import netbox [path]";

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
