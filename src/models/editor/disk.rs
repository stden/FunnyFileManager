//! Purpose: What the editor remembers about the file on disk, so a save can
//! tell whether another program changed it since, and the question the save
//! asks when one did.

use std::hash::Hasher;
use std::num::NonZeroU64;

use crate::models::Confirm;

/// A fingerprint of a file's bytes, as the editor last read or wrote them.
///
/// Of the content rather than the modification time: a timestamp can miss a
/// same-size write within the filesystem's time granularity, and it moves on
/// a `touch` that changed nothing. Never zero, so the editor's
/// `Option<Fingerprint>` costs no more than the hash: the editor is the
/// largest overlay, and every byte it grows by is paid by all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint(NonZeroU64);

impl Fingerprint {
    /// The fingerprint of a file holding `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        hasher.write(bytes);
        Self(NonZeroU64::new(hasher.finish()).unwrap_or(NonZeroU64::MIN))
    }
}

/// What the editor does once a save the user confirmed has been written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterSave {
    /// F2: keep editing.
    Stay,
    /// Save in the exit dialog: close the editor.
    Close,
}

/// "The file changed on disk. Overwrite?" — asked by a save that found
/// another program's write, with the cursor on one of its two buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverwritePrompt {
    /// What the save was going to do next.
    pub then: AfterSave,
    /// `Yes` is Overwrite, `No` is Cancel.
    pub answer: Confirm,
}

impl OverwritePrompt {
    /// The question, with the cursor on Cancel: a reflexive Enter must not
    /// throw away the other program's work.
    #[must_use]
    pub const fn new(then: AfterSave) -> Self {
        Self {
            then,
            answer: Confirm::No,
        }
    }
}
