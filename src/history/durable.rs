//! Purpose: The durability gate — which command lines may be written to disk.

use crate::command::Command;

/// A command line proven safe to persist.
///
/// The only way to obtain one is [`DurableLine::classify`], which returns
/// `None` for external commands, `Command::Invalid`, and any line carrying an
/// embedded newline. `History::save` writes exclusively from `self.durable`, a
/// `VecDeque<DurableLine>`, so a secret cannot reach disk: unclassified text is
/// not even representable in the type the writer reads from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableLine(String);

impl DurableLine {
    /// Classify `line`; `Some` iff it parses to a known verb.
    #[must_use]
    pub fn classify(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        // Invariant: one `DurableLine` is exactly one physical file line.
        // `save()` writes one line per entry and `load()` re-classifies per
        // physical line, so an interior '\n'/'\r' would let the parse gate
        // (which sees the whole string) disagree with the persistence unit —
        // a completed filename containing a newline could smuggle a second
        // physical line that reloads as a `Shell` command. Reject outright.
        if trimmed.contains(['\n', '\r']) {
            return None;
        }
        (durability(&Command::parse(trimmed)) == Durability::Durable)
            .then(|| Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a parsed line may reach disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Durability {
    Durable,
    SessionOnly,
}

const fn durability(command: &Command) -> Durability {
    match command {
        // Allow-list, deliberately with no wildcard arm: every known
        // verb is named explicitly, so adding a new `Command` variant
        // is a compile error here until a human decides whether it can
        // carry secrets. The gate fails closed on unrecognized variants
        // instead of persisting them by default.
        Command::Cd(_)
        | Command::Pwd
        | Command::Sort(_)
        | Command::BookmarkSet(_)
        | Command::BookmarkGo(_)
        | Command::MarkAll
        | Command::UnmarkAll
        | Command::Help
        | Command::Stats
        | Command::WinMsg
        | Command::Wsl
        | Command::WinPerf
        | Command::Mcp
        | Command::Lsp
        | Command::DiskCleanup
        | Command::Sessions(_)
        | Command::Quit => Durability::Durable,
        // Shell fall-through and known-verb-bad-args can carry secrets
        // (raw user text) — never durable. `Empty` is unreachable here
        // (`classify` returns before parsing a blank line) but is named rather
        // than wildcarded, so a new variant stays a compile error.
        Command::Shell(_) | Command::NewConsole(_) | Command::Invalid { .. } | Command::Empty => {
            Durability::SessionOnly
        }
    }
}
