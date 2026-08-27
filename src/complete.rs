//! Purpose: Tab-completion for the `:` command line (verbs, sub-commands, paths).
//!
//! Verb and sub-command completion is table-driven; `cd` path completion
//! delegates to the **fish** shell's completion engine when `fish` is on `PATH`
//! (the intended setup on macOS), and falls back to a built-in directory scan
//! so completion still works — and stays testable — without fish installed.

use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::command::{expand_tilde, VERBS};
use crate::models::{Candidate, CandidateKind};

/// Bound on a fish completion spawn so a hung fish cannot freeze the TUI.
/// Shell commands use a longer cap; Tab must stay snappy and fall back to the
/// built-in scan when fish is slow or stuck.
const FISH_COMPLETE_TIMEOUT: Duration = Duration::from_millis(1500);

// Test override for the completion shell, following the crate's cfg-gated
// injection-point convention: the poll loop's slow path only shows itself
// when the child outlives a poll tick, which real `fish` rarely does.
// Thread-local so a test's stub cannot leak into tests running in parallel.
#[cfg(any(test, tarpaulin))]
thread_local! {
    pub(crate) static FISH_PROGRAM_FOR_TESTS: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// The program spoken to as fish: the real one, unless a test injected a stub.
fn fish_program() -> PathBuf {
    #[cfg(any(test, tarpaulin))]
    if let Some(program) = FISH_PROGRAM_FOR_TESTS.with(|slot| slot.borrow().clone()) {
        return program;
    }
    PathBuf::from("fish")
}

pub mod execs;
pub mod matcher;
pub mod shellwords;

/// Sources a completion draws on beyond the raw input.
///
/// `cwd` is the active panel's directory; `bookmarks` are its slots (index =
/// slot number, `None` = empty). Phase 3 adds `bookmarks`; earlier phases
/// used only `cwd`.
pub struct CompletionContext<'a> {
    pub cwd: &'a Path,
    pub bookmarks: &'a [Option<PathBuf>],
}

impl<'a> CompletionContext<'a> {
    /// Context with no bookmark slots — for positions that never complete a
    /// bookmark, and for tests that do not exercise bookmarks.
    #[must_use]
    pub const fn for_cwd(cwd: &'a Path) -> Self {
        Self {
            cwd,
            bookmarks: &[],
        }
    }
}

/// Outcome of a completion attempt.
pub struct Completion {
    /// The full replacement text for the command line.
    pub line: String,
    /// All matching candidates (len > 1 means the result was ambiguous and only
    /// advanced to the longest common prefix — useful to surface to the user).
    pub candidates: Vec<Candidate>,
}

/// Complete `input` (the current command-line text) given the active panel's
/// directory `cwd`. Returns `None` when there is nothing to complete.
#[must_use]
pub fn complete(input: &str, cwd: &Path) -> Option<Completion> {
    complete_with(input, &CompletionContext::for_cwd(cwd))
}

/// Complete `input` (the current command-line text) against `ctx`. Returns
/// `None` when there is nothing to complete.
#[must_use]
pub fn complete_with(input: &str, ctx: &CompletionContext) -> Option<Completion> {
    let (head, token) = split_target(input);
    let head_words: Vec<&str> = head.split_whitespace().collect();

    let candidates = match head_words.first() {
        // Completing the command name itself (first word, nothing before it).
        None => command_candidates(token, ctx.cwd),
        // `cd` takes the whole rest of the line as one path and is parsed with
        // quote stripping rather than escapes, so it keeps the raw token.
        Some(&"cd") => plain_candidates(path_complete(token, ctx.cwd), CandidateKind::Path),
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
        // A built-in verb in a position it does not model: nothing to offer,
        // and guessing a path there would be worse than silence.
        Some(verb) if VERBS.contains(verb) => return None,
        // Everything else is a shell command, and every argument of one is a
        // path until proven otherwise — which is what makes `cat src/ma<Tab>`
        // and `ls /usr/l<Tab>` work at all.
        _ => shell_path_candidates(token, ctx.cwd),
    };

    if candidates.is_empty() {
        return None;
    }

    let completed = if candidates.len() == 1 {
        finish_word(&candidates[0].text, candidates[0].kind)
    } else {
        // Ambiguous: advance to the longest common prefix only (no trailing space).
        let texts: Vec<String> = candidates.iter().map(|c| c.text.clone()).collect();
        let lcp = longest_common_prefix(&texts);
        if lcp.len() <= token.len() {
            // Already at the common prefix — nothing to insert, just report options.
            return Some(Completion {
                line: input.to_string(),
                candidates,
            });
        }
        lcp
    };

    Some(Completion {
        line: format!("{head}{completed}"),
        candidates,
    })
}

/// Candidates for the first word of a line: what could be run.
///
/// A token holding a path separator, or starting with `.` or `~`, is a program
/// named by location (`./build.sh`, `~/bin/tool`) and completes as a path.
/// Otherwise the built-in verbs and the `$PATH` executables are offered
/// together — the verbs fuzzily as they always have been, the executables by
/// prefix (see [`execs::matching`] for why the two rules differ).
fn command_candidates(token: &str, cwd: &Path) -> Vec<Candidate> {
    let literal = shellwords::unescape(token);
    if looks_like_a_path(&literal) {
        return shell_path_candidates(token, cwd);
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
fn looks_like_a_path(token: &str) -> bool {
    token.contains('/') || token.starts_with('.') || token.starts_with('~')
}

/// Path candidates for an argument of a shell command, ready to splice back.
///
/// The token is unescaped before it is matched (the user may have typed
/// `My\ Doc`) and every candidate is escaped before it is offered, so a name
/// holding a space reaches `sh -c` as one argument instead of two. `cd` skips
/// all of this: it is parsed by [`crate::command::Command::parse`], which
/// strips quotes instead of resolving escapes.
fn shell_path_candidates(token: &str, cwd: &Path) -> Vec<Candidate> {
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
fn is_home_syntax(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('~') else {
        return false;
    };
    rest.is_empty() || rest.contains('/')
}

/// Undo [`shellwords::escape`]'s leading-`~` backslash when the tilde is the
/// user's own expansion syntax rather than the first letter of a file name.
fn keep_typed_tilde(escaped: String, typed_tilde: bool) -> String {
    if !typed_tilde {
        return escaped;
    }
    match escaped.strip_prefix("\\~") {
        Some(rest) => format!("~{rest}"),
        None => escaped,
    }
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

/// Split `input` into the fixed prefix and the token being completed.
///
/// `cd` is the exception: it takes the whole rest of the line as ONE path
/// argument (see [`crate::command::Command::parse`]), so a directory name
/// containing spaces completes as a unit and parses back the same way.
/// `pub(crate)` because the dropdown's `Enter`-accept in [`crate::popup`] must
/// replace exactly the token this split identified.
pub(crate) fn split_target(input: &str) -> (&str, &str) {
    match input.trim_start().split_once(char::is_whitespace) {
        // `token` is a suffix of `input`, so its length locates the split.
        Some(("cd", rest)) => {
            let token = rest.trim_start();
            (&input[..input.len() - token.len()], token)
        }
        // Quote- and escape-aware, so `cat My\ Doc` completes against
        // `My Doc` rather than against `Doc`.
        _ => shellwords::last_word(input),
    }
}

/// Keep the options matching `token`, best first (see [`matcher::rank`]).
fn match_filter(options: &[&str], token: &str) -> Vec<String> {
    matcher::rank(token, options.iter().map(|&o| o.to_string()).collect())
}

/// Append a trailing space when accepting a candidate, except for directory
/// paths (ending in `/`) where the user likely wants to descend. Shared by
/// `Tab`'s lone-candidate accept and the dropdown's `Enter` accept, so both
/// finish a word the same way.
pub(crate) fn finish_word(candidate: &str, kind: CandidateKind) -> String {
    if kind == CandidateKind::Path && candidate.ends_with('/') {
        candidate.to_string()
    } else {
        format!("{candidate} ")
    }
}

/// Longest common prefix of a non-empty candidate list, on `char` boundaries.
fn longest_common_prefix(candidates: &[String]) -> String {
    let first = &candidates[0];
    let mut prefix_len = first.len();
    for other in &candidates[1..] {
        prefix_len = common_prefix_len(&first[..prefix_len], other);
    }
    first[..prefix_len].to_string()
}

/// Byte length of the shared leading run of characters in `a` and `b`.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .map(|(ca, _)| ca.len_utf8())
        .sum()
}

/// Complete a `cd` path argument by merging both engines.
///
/// The built-in fuzzy scan always contributes: fish only offers prefix
/// matches, and the design promises `dwn` matches `Downloads/` whether or
/// not fish is installed. fish's own candidates are merged in on top — they
/// add what a directory scan cannot know (case-corrected fallbacks, `CDPATH`
/// hits) — rather than being trusted exclusively.
fn path_complete(token: &str, cwd: &Path) -> Vec<String> {
    let (dir_part, leaf) = split_token(token);
    let mut pool: Vec<String> = builtin_path_complete(dir_part, leaf, cwd)
        .into_iter()
        .map(|l| format!("{dir_part}{l}"))
        .collect();
    // fish completes the last shell *word*, so a token holding a name with
    // spaces would come back stripped of its prefix — skip fish for those.
    if !token.contains(char::is_whitespace) {
        if let Some(fish) = fish_path_complete(token, cwd) {
            // Candidate lists are one directory's entries — linear dedup is fine.
            for candidate in fish {
                if !pool.contains(&candidate) {
                    pool.push(candidate);
                }
            }
        }
    }
    rank_by_leaf(leaf, pool)
}

/// Split a `cd` token into its already-typed directory part (ending in `/`,
/// or empty) and the partial leaf name being completed.
fn split_token(token: &str) -> (&str, &str) {
    match token.rfind('/') {
        Some(i) => (&token[..=i], &token[i + 1..]),
        None => ("", token),
    }
}

/// Split a path completion *candidate* (not a token being typed) into its
/// directory part and leaf, ignoring the candidate's own trailing directory
/// slash when finding the split point — so `"sub/handlers/"` splits as
/// `("sub/", "handlers/")`, not `("sub/handlers/", "")`.
fn split_leaf(candidate: &str) -> (&str, &str) {
    let search_len = candidate
        .strip_suffix('/')
        .map_or(candidate.len(), str::len);
    match candidate[..search_len].rfind('/') {
        Some(i) => candidate.split_at(i + 1),
        None => ("", candidate),
    }
}

/// Rank full path candidates by each one's own leaf (see [`split_leaf`]).
///
/// Scoring the leaf alone — never the `dir_part`-prefixed full path — keeps
/// an exact leaf-prefix match (e.g. `"complete_b"` for query `"com"`) in the
/// `Prefix` tier even deep inside a subdirectory, and keeps shared directory
/// text from polluting point scoring. Candidates whose leaf does not match
/// are appended after the ranked ones, in their incoming relative order,
/// never dropped: fish validated them (case-insensitive fallbacks score
/// `None` under [`matcher::score`]'s smartcase rule but are still real
/// completions).
fn rank_by_leaf(leaf: &str, candidates: Vec<String>) -> Vec<String> {
    let (matching, mut rest): (Vec<String>, Vec<String>) = candidates
        .into_iter()
        .partition(|c| matcher::score(leaf, split_leaf(c).1).is_some());

    // Rank by leaf text via `matcher::rank`, then look each ranked leaf back
    // up to its original candidate — queued per leaf text, in first-seen
    // order, in case two candidates happen to share a leaf name.
    let mut by_leaf: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut leaves = Vec::with_capacity(matching.len());
    for candidate in matching {
        let leaf_text = split_leaf(&candidate).1.to_string();
        by_leaf
            .entry(leaf_text.clone())
            .or_default()
            .push_back(candidate);
        leaves.push(leaf_text);
    }

    let mut ranked: Vec<String> = matcher::rank(leaf, leaves)
        .into_iter()
        .filter_map(|leaf_text| by_leaf.get_mut(&leaf_text)?.pop_front())
        .collect();

    ranked.append(&mut rest);
    ranked
}

/// Ask the fish shell for `cd <token>` completions, run with `cwd` as the
/// working directory. Returns `None` when fish is unavailable, times out, or
/// yields nothing, so the caller can fall back. fish prints one candidate per
/// line as `value\tdescription`; we keep the value (the full token replacement).
fn fish_path_complete(token: &str, cwd: &Path) -> Option<Vec<String>> {
    let line = format!("cd {token}");
    // Pass the line via an env var so fish does the quoting, not us.
    let mut child = Command::new(fish_program())
        .current_dir(cwd)
        .env("FFM_COMPLETE_LINE", &line)
        .args(["-c", "complete --do-complete=\"$FFM_COMPLETE_LINE\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + FISH_COMPLETE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut stdout = Vec::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_end(&mut stdout);
                }
                let candidates = parse_fish_completions(&String::from_utf8_lossy(&stdout));
                return (!candidates.is_empty()).then_some(candidates);
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            // Timed out — or `try_wait` itself failed, which gets the same
            // treatment: kill whatever is left and fall back to no completion.
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Parse `fish --do-complete` output: one candidate per line as
/// `value\tdescription`. Keep the value column (the full token replacement),
/// drop the description, and skip blank lines. Split out from
/// [`fish_path_complete`] so the parsing is testable without spawning fish.
fn parse_fish_completions(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|l| l.split('\t').next())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect()
}

/// Built-in `cd` path completion: list entries of the directory implied by
/// `dir_part` whose names match `leaf` (see [`matcher::score`]). Directory
/// matches gain a trailing `/`. Returns bare leaves — no `dir_part` prefix —
/// which the caller reattaches before ranking; see [`rank_by_leaf`].
fn builtin_path_complete(dir_part: &str, leaf: &str, cwd: &Path) -> Vec<String> {
    let search_dir = resolve_dir(dir_part, cwd);

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            matcher::score(leaf, &name)?;
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            let suffix = if is_dir { "/" } else { "" };
            Some(format!("{name}{suffix}"))
        })
        .collect()
}

/// Resolve the directory referenced by a path's directory part, expanding a
/// leading `~` and joining relative parts onto `cwd`. Tilde expansion goes
/// through [`expand_tilde`], the same one `cd` uses, so completion cannot offer
/// a directory `cd` then fails to open.
fn resolve_dir(dir_part: &str, cwd: &Path) -> PathBuf {
    let p = expand_tilde(Path::new(dir_part));
    if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    }
}

#[cfg(test)]
mod tests;
