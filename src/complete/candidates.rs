//! Purpose: which candidates each command-line position offers — verbs, sub-commands, `$PATH` programs, bookmark slots and shell paths.

use std::path::{Path, PathBuf};

use crate::command::VERBS;
use crate::models::{Candidate, CandidateKind};

use super::paths::path_complete;
use super::{execs, matcher, shellwords, CompletionContext, PathSyntax};

mod shell_paths;

use shell_paths::shell_path_candidates;

pub(super) fn candidates_for(
    syntax: PathSyntax,
    head: &str,
    token: &str,
    ctx: &CompletionContext<'_>,
) -> Vec<Candidate> {
    let head_words: Vec<&str> = head.split_whitespace().collect();

    match head_words.first() {
        // Completing the command name itself (first word, nothing before it).
        None => command_candidates(syntax, token, ctx.cwd),
        // `cd` takes the whole rest of the line as one path and is parsed with
        // quote stripping rather than escapes, so it keeps the raw token.
        Some(&"cd") => plain_candidates(path_complete(syntax, token, ctx.cwd), CandidateKind::Path),
        Some(&"sort") if head_words.len() == 1 => plain_candidates(
            match_filter(&["name", "extension", "size"], token),
            CandidateKind::Word,
        ),
        Some(&"bookmark") if head_words.len() == 1 => {
            plain_candidates(match_filter(&["set", "go"], token), CandidateKind::Word)
        }
        Some(&"bookmark") if head_words.len() == 2 => match head_words.get(1) {
            Some(&"go") => bookmark_slot_candidates(ctx.bookmarks, token),
            // `set` may target any slot, so keep the full digit list.
            _ => plain_candidates(
                match_filter(&["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"], token),
                CandidateKind::Word,
            ),
        },
        Some(&"mark") if head_words.len() == 1 => {
            plain_candidates(match_filter(&["all"], token), CandidateKind::Word)
        }
        Some(&"sessions") => sessions_candidates(syntax, &head_words, token, ctx),
        // A built-in verb in a position it does not model: nothing to offer,
        // and guessing a path there would be worse than silence.
        Some(verb) if VERBS.contains(verb) => Vec::new(),
        // Everything else is a shell command, and every argument of one is a
        // path until proven otherwise — which is what makes `cat src/ma<Tab>`
        // and `ls /usr/l<Tab>` work at all.
        _ => shell_path_candidates(syntax, token, ctx.cwd),
    }
}

/// `sessions import netbox <path>`: the subcommand, the source, then the
/// file, whole-rest like `cd`'s. A word out of place offers nothing.
fn sessions_candidates(
    syntax: PathSyntax,
    head_words: &[&str],
    token: &str,
    ctx: &CompletionContext<'_>,
) -> Vec<Candidate> {
    match head_words {
        [_] => plain_candidates(match_filter(&["import"], token), CandidateKind::Word),
        [_, "import"] => plain_candidates(match_filter(&["netbox"], token), CandidateKind::Word),
        [_, "import", "netbox", ..] => {
            plain_candidates(path_complete(syntax, token, ctx.cwd), CandidateKind::Path)
        }
        _ => Vec::new(),
    }
}

/// Candidates for the first word of a line: what could be run.
///
/// A token holding a path separator, or starting with `.` or `~`, is a program
/// named by location (`./build.sh`, `~/bin/tool`) and completes as a path.
/// Otherwise the built-in verbs and the `$PATH` executables are offered
/// together — the verbs fuzzily as they always have been, the executables by
/// prefix (see [`execs::matching`] for why the two rules differ).
fn command_candidates(syntax: PathSyntax, token: &str, cwd: &Path) -> Vec<Candidate> {
    let literal = shellwords::unescape(syntax, token);
    if looks_like_a_path(syntax, &literal) {
        return shell_path_candidates(syntax, token, cwd);
    }

    let verbs = match_filter(VERBS, &literal);
    // A prefix that picks out exactly one built-in verb *is* the answer: the
    // verbs are this command line's own vocabulary, and `Command::parse` will
    // take that word whatever `$PATH` also happens to offer. Padding the list
    // with `pwdx` next to `pwd` would turn an accepted keystroke into a menu.
    if verbs.len() == 1 {
        return plain_candidates(verbs, CandidateKind::Word);
    }

    // Ambiguous among the verbs, or matching none of them: `$PATH` joins in,
    // always *after* the verbs, so a built-in can never be outranked by a
    // program that merely happens to have a shorter name.
    let mut pool = verbs;
    for executable in execs::matching(&literal) {
        if !pool.contains(&executable) {
            pool.push(executable);
        }
    }
    plain_candidates(pool, CandidateKind::Word)
}

/// Whether a first word names a program by location rather than by name.
fn looks_like_a_path(syntax: PathSyntax, token: &str) -> bool {
    token.contains(|ch| syntax.is_separator(ch)) || token.starts_with('.') || token.starts_with('~')
}

/// Candidates for `bookmark go <slot>`: only the slots that hold a path, each
/// carrying that path as its hint, filtered by the digit typed so far and
/// ranked like any other word candidate.
fn bookmark_slot_candidates(bookmarks: &[Option<PathBuf>], token: &str) -> Vec<Candidate> {
    let populated: Vec<(String, String)> = bookmarks
        .iter()
        .enumerate()
        .filter_map(|(i, slot)| {
            let path = slot.as_ref()?;
            Some((i.to_string(), path.display().to_string()))
        })
        .collect();

    // Rank the slot digits through the shared matcher so ordering matches every
    // other source, then reattach each slot's path as its hint.
    let digits: Vec<String> = populated.iter().map(|(d, _)| d.clone()).collect();
    matcher::rank(token, digits)
        .into_iter()
        .map(|digit| {
            let hint = populated
                .iter()
                .find(|(d, _)| *d == digit)
                .map(|(_, p)| p.clone());
            Candidate {
                text: digit,
                kind: CandidateKind::Word,
                hint,
            }
        })
        .collect()
}

/// Wrap ranked plain strings as word/path candidates without hints.
fn plain_candidates(texts: Vec<String>, kind: CandidateKind) -> Vec<Candidate> {
    texts
        .into_iter()
        .map(|text| Candidate {
            text,
            kind,
            hint: None,
        })
        .collect()
}

/// Keep the options matching `token`, best first (see [`matcher::rank`]).
fn match_filter(options: &[&str], token: &str) -> Vec<String> {
    matcher::rank(token, options.iter().map(|&o| o.to_owned()).collect())
}

#[cfg(test)]
mod tests;
