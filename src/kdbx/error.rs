//! Purpose: What can go wrong in the `.kdbx` overlay, as messages that are safe
//! to show — none of them carries the master password.

use crate::keepass::KeepassError;

/// Failures a caller of the unlocked API can hit.
///
/// Distinct from [`KeepassError`] so "the vault is locked" is not misreported as
/// "entry not found". No variant carries the master password.
#[derive(Debug)]
pub enum KdbxError {
    /// The vault is locked, so there is nothing to read.
    Locked,
    /// Nothing is under the cursor — usually a filter that matches no entry.
    NoSelection,
    NoEdit,
    NothingToUndo,
    ReadOnly,
    /// The underlying `.kdbx` read failed.
    Keepass(KeepassError),
}

impl std::fmt::Display for KdbxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locked => write!(f, "Vault is locked"),
            Self::NoSelection => write!(f, "No entry selected"),
            Self::NoEdit => write!(f, "No entry is being edited"),
            Self::NothingToUndo => write!(f, "No KDBX change to undo"),
            Self::ReadOnly => write!(f, "View mode: open the database with F4 to edit entries"),
            Self::Keepass(error) => write!(f, "Cannot read entry: {error}"),
        }
    }
}

impl std::error::Error for KdbxError {}

/// Turn an unlock failure into a message that is safe to display.
///
/// Mapped per variant rather than forwarded verbatim: `KeepassError::Open`
/// wraps a third-party crate's text, and this string is shown in the UI and may
/// be near logging code, so nothing derived from the credentials is allowed
/// through. The wrong-password case is also merged with the corrupt-file case
/// on purpose — the two are indistinguishable to a caller anyway.
pub(super) fn unlock_error_message(error: &KeepassError) -> String {
    match error {
        KeepassError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
            "File not found".to_owned()
        }
        KeepassError::Io(io) => format!("Cannot read file: {io}"),
        KeepassError::Open(_) => "Wrong password or key file".to_owned(),
        // Save / EntryNotFound / AlreadyExists / Serialize cannot arise from a
        // read, but their Display text is fixed and secret-free.
        other => other.to_string(),
    }
}
