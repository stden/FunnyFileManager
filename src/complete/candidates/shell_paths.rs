//! Purpose: Path candidates for an argument of a shell command — unescaped to
//! match, escaped to splice back, a typed `~` kept as home syntax.

use std::path::Path;

use crate::complete::paths::path_complete;
use crate::complete::shellwords;
use crate::models::{Candidate, CandidateKind};

/// Path candidates for an argument of a shell command, ready to splice back.
///
/// The token is unescaped before it is matched (the user may have typed
/// `My\ Doc`) and every candidate is escaped before it is offered, so a name
/// holding a space reaches `sh -c` as one argument instead of two. `cd` skips
/// all of this: it is parsed by [`crate::command::Command::parse`], which
/// strips quotes instead of resolving escapes.
pub(super) fn shell_path_candidates(token: &str, cwd: &Path) -> Vec<Candidate> {
    let literal = shellwords::unescape(token);
    // A leading `~` the *user* typed is tilde expansion, and the path source
    // keeps that prefix on every candidate it returns. Escaping it would hand
    // the shell `\~/Documents` — a literal directory named `~`, which does not
    // exist — so `cat ~/Doc<Tab>` would complete into a command that cannot
    // work. A candidate that merely happens to *start* with `~` (a file called
    // `~backup`) is still escaped, because there the tilde is part of a name.
    let typed_tilde = is_home_syntax(&literal);
    path_complete(&literal, cwd)
        .into_iter()
        .map(|path| Candidate {
            text: keep_typed_tilde(shellwords::escape(&path), typed_tilde),
            kind: CandidateKind::Path,
            hint: None,
        })
        .collect()
}

/// Whether a token's leading `~` is home-directory syntax rather than the first
/// letter of a file name.
///
/// `~` and `~user` are both valid expansion syntax to a shell, so the token
/// alone cannot always say which was meant. The slash decides it: `~/x` and
/// `~user/x` name a place inside a home directory, while a bare `~backup` is
/// overwhelmingly a file in the current directory — and a file is the thing
/// this completer just found on disk.
pub(in crate::complete) fn is_home_syntax(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('~') else {
        return false;
    };
    rest.is_empty() || rest.contains('/')
}

/// Undo [`shellwords::escape`]'s leading-`~` backslash when the tilde is the
/// user's own expansion syntax rather than the first letter of a file name.
pub(in crate::complete) fn keep_typed_tilde(escaped: String, typed_tilde: bool) -> String {
    if !typed_tilde {
        return escaped;
    }
    match escaped.strip_prefix("\\~") {
        Some(rest) => format!("~{rest}"),
        None => escaped,
    }
}
