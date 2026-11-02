//! Purpose: `Command::parse` — the single source of command syntax, turning a
//! raw command line into a typed `Command`.

use std::path::PathBuf;

use super::{
    Command, SessionsCommand, ShellCommand, BOOKMARK_USAGE, CD_USAGE, MARK_USAGE, SESSIONS_USAGE,
    SORT_USAGE,
};
use crate::models::{BookmarkSlot, SortMode};

impl Command {
    /// A known verb used with arguments it does not accept.
    fn invalid(input: &str, usage: &'static str) -> Self {
        Self::Invalid {
            input: input.to_owned(),
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
            "sessions" => Self::parse_sessions(input, &mut parts),
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

    fn parse_sessions(input: &str, parts: &mut std::str::SplitWhitespace<'_>) -> Self {
        match (parts.next(), parts.next()) {
            (None, _) => Self::Sessions(SessionsCommand::Open),
            (Some("import"), Some("netbox")) => {
                Self::Sessions(SessionsCommand::ImportNetBox(rest_after(input, 3)))
            }
            _ => Self::invalid(input, SESSIONS_USAGE),
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
    rest_after(input, 1)
}

/// Everything after the first `words` whitespace-separated words, quotes
/// stripped, as a path — `None` when nothing follows. Whole-rest rather than
/// one token for the reason [`cd_argument`] gives; `sessions import netbox`
/// takes its path the same way.
fn rest_after(input: &str, words: usize) -> Option<PathBuf> {
    let mut rest = input.trim_start();
    for _ in 0..words {
        let (_, tail) = rest.split_once(char::is_whitespace)?;
        rest = tail.trim_start();
    }
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
