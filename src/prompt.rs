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

mod identity;

pub use identity::{Identity, Privilege};

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

    push(
        crate::path_display::label(directory).into_owned(),
        Part::Path,
    );

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

#[cfg(test)]
mod tests;
