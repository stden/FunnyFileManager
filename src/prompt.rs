//! Purpose: The shell-style command prompt — `● root@host /srv/app (main) #`.
//!
//! Built as named segments rather than one formatted string, because each part
//! is a different colour and the renderer is the only thing that should know
//! which. Assembling the text here and painting it there also means the width
//! is measured once, on the same pieces that get drawn.
//!
//! Modelled on Starship, and like Starship it reads rather than runs: the
//! branch comes out of `.git/HEAD` (see [`crate::git`]), the user and host out
//! of the environment. A prompt is redrawn on every keystroke, so anything that
//! spawns a process is out of the question.

use std::path::Path;

/// What a piece of the prompt is, which is what decides its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The leading bullet: green when the last command succeeded, red when it
    /// did not. Starship's most useful segment, and free — the answer is
    /// already known.
    StatusOk,
    StatusFailed,
    User,
    /// The `@` between user and host.
    At,
    Host,
    Path,
    /// The git branch, in brackets.
    Branch,
    /// `#` for root, `$` otherwise — the oldest convention there is for "this
    /// shell can overwrite anything".
    Sigil,
    /// The blanks between segments.
    Space,
}

/// One piece of the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub part: Part,
}

/// Whether the last command the user ran worked.
///
/// A `bool` would read as "true is good" at some call sites and "true is a
/// failure" at others; these two names cannot be confused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LastCommand {
    #[default]
    Succeeded,
    Failed,
}

impl LastCommand {
    /// Read the bullet's colour off the status the last command left behind.
    ///
    /// Only outright failures count. `Cleared` and `KeyHints` are not outcomes,
    /// and a red bullet on a fresh session would say something untrue.
    #[must_use]
    pub const fn from_status(status: &crate::status::Status) -> Self {
        use crate::status::Status;
        match status {
            Status::SaveFailed { .. } | Status::RefreshFailed(_) => Self::Failed,
            _ => Self::Succeeded,
        }
    }
}

/// Whether this shell can overwrite anything, which is what `#` means.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Privilege {
    #[default]
    User,
    Root,
}

/// Who the prompt says you are, and on which machine.
///
/// Passed in rather than read inside [`segments`] so the prompt can be tested
/// against a known identity. Reading the environment *inside* the assembly made
/// every one of these values untestable: mutation testing showed `user`, `host`
/// and `is_root` could each return anything at all — including the wrong
/// sigil, which is the one that matters — without a single test noticing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub user: Option<String>,
    pub host: Option<String>,
    pub privilege: Privilege,
}

impl Identity {
    /// Read the identity of the process actually running.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            user: user(),
            host: host(),
            privilege: if is_root() {
                Privilege::Root
            } else {
                Privilege::User
            },
        }
    }
}

/// Assemble the prompt for `directory`, for whoever is running.
#[must_use]
pub fn segments(directory: &Path, last: LastCommand) -> Vec<Segment> {
    segments_for(directory, last, &Identity::from_env())
}

/// Assemble the prompt for a given identity.
#[must_use]
pub fn segments_for(directory: &Path, last: LastCommand, identity: &Identity) -> Vec<Segment> {
    let mut prompt = Vec::new();
    let mut push = |text: String, part: Part| prompt.push(Segment { text, part });

    push(
        "●".to_owned(),
        match last {
            LastCommand::Succeeded => Part::StatusOk,
            LastCommand::Failed => Part::StatusFailed,
        },
    );
    push(" ".to_owned(), Part::Space);

    append_identity(&mut push, identity);

    push(directory.display().to_string(), Part::Path);

    if let Some(branch) = crate::git::branch(directory) {
        push(" ".to_owned(), Part::Space);
        push(format!("({branch})"), Part::Branch);
    }

    push(" ".to_owned(), Part::Space);
    push(
        match identity.privilege {
            Privilege::Root => "#",
            Privilege::User => "$",
        }
        .to_owned(),
        Part::Sigil,
    );
    push(" ".to_owned(), Part::Space);
    prompt
}

fn append_identity(push: &mut impl FnMut(String, Part), identity: &Identity) {
    if let Some(user) = identity.user.clone() {
        push(user, Part::User);
        if let Some(host) = identity.host.clone() {
            push("@".to_owned(), Part::At);
            push(host, Part::Host);
        }
        push(" ".to_owned(), Part::Space);
    }
}

/// Variables that may carry the login name, in the order they are trusted.
const USER_VARS: [&str; 3] = ["USER", "LOGNAME", "USERNAME"];

/// The login name, from the environment.
fn user() -> Option<String> {
    let values: Vec<Option<String>> = USER_VARS
        .iter()
        .map(|name| std::env::var(name).ok())
        .collect();
    pick_user(&values)
}

/// The first candidate that is actually a name.
///
/// Split from the reading so it can be tested: an empty variable is worse than
/// an unset one, because it renders as a bare `@host` — and a function that
/// reads the environment can be given no input at all by a test, which is how
/// this rule went unchecked until mutation testing pointed at it.
fn pick_user(candidates: &[Option<String>]) -> Option<String> {
    candidates
        .iter()
        .flatten()
        .find(|name| !name.is_empty())
        .cloned()
}

/// The machine's name.
///
/// `/etc/hostname` before `$HOSTNAME`: the variable is not exported by every
/// shell, so on many systems it is simply absent.
fn host() -> Option<String> {
    let file = std::fs::read_to_string("/etc/hostname").ok();
    let variable = std::env::var("HOSTNAME").ok();
    pick_host(file.as_deref(), variable.as_deref())
}

/// The machine's name, from the file if it says one and the variable if not.
///
/// The file wins because it is what the system believes; the variable is a
/// shell's copy of it and is often stale or unset. Both are trimmed, and an
/// empty result from either is treated as absent rather than shown.
fn pick_host(file: Option<&str>, variable: Option<&str>) -> Option<String> {
    [file, variable]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

/// Whether this process can overwrite anything, which is what `#` means.
#[cfg(unix)]
fn is_root() -> bool {
    // SAFETY: `geteuid` takes no arguments, touches no memory and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    false
}

#[cfg(test)]
mod tests;
