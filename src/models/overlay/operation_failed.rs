//! Purpose: A copy, move or delete that stopped at its first failure: what it
//! was doing, where it failed, why, how far it had got, and whether it can
//! be tried again as an administrator.

use std::path::PathBuf;

use super::Confirm;
use crate::progress::Operation;
use crate::status::FailedEntry;

/// Far's "Cannot copy the file" dialog.
///
/// The operation stops at the first entry it cannot copy, move or delete
/// rather than leave the user to find out from a count in the status line
/// which of a thousand files went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationFailed {
    operation: Operation,
    failed: FailedEntry,
    /// Why, when the error alone does not say.
    denial: Option<Denial>,
    /// Selected entries done before the failure.
    done: usize,
    /// Selected entries not started because of it.
    left: usize,
    /// What an administrator could try again, when that might help.
    retry: Option<ElevatedRetry>,
    /// Yes: the "Retry as administrator" button has the cursor; No: OK.
    focus: Confirm,
}

/// Why the system refused, in words the user can act on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Denial {
    /// Another program has it open (Windows' sharing violation).
    InUse,
    /// It carries the read-only attribute.
    ReadOnly,
    /// The account has no rights to it, and FFM does not run elevated.
    NeedsAdministrator,
    /// Refused although FFM runs as an administrator.
    NotEvenAdministrator,
    /// The folder holding it takes no change from this user.
    FolderLocked(PathBuf),
    /// Another user owns it.
    OtherOwner,
    /// Its access rights do not allow it; nothing more is known.
    Rights,
}

impl Denial {
    /// One sentence for the dialog.
    #[must_use]
    pub fn explanation(&self) -> String {
        match self {
            Self::InUse => "It is open in another program: close it and try again.".to_owned(),
            Self::ReadOnly => "It is read-only.".to_owned(),
            Self::NeedsAdministrator => {
                "Your account has no rights to it; an administrator may have them.".to_owned()
            }
            Self::NotEvenAdministrator => {
                "Its access list denies even an administrator.".to_owned()
            }
            Self::FolderLocked(folder) => format!(
                "The folder {} does not let you change what it holds.",
                crate::path_display::label(folder)
            ),
            Self::OtherOwner => "It belongs to another user.".to_owned(),
            Self::Rights => "Its access rights do not allow it.".to_owned(),
        }
    }
}

/// The part of a stopped operation an administrator could redo: the entry
/// that failed and those not started, with what the operation was told.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ElevatedRetry {
    pub operation: Operation,
    pub sources: Vec<PathBuf>,
    /// The folder a typed destination is relative to.
    pub base: PathBuf,
    /// How many entries the operation was first given: a typed destination
    /// for several is a folder, for one a new name, and the retry must read
    /// it as the first try did.
    pub selected: usize,
    /// The F5/F6 dialog's settings; `None` for a delete.
    pub settings: Option<crate::copy::CopyPrompt>,
}

impl OperationFailed {
    #[must_use]
    pub const fn new(operation: Operation, failed: FailedEntry, done: usize, left: usize) -> Self {
        Self {
            operation,
            failed,
            denial: None,
            done,
            left,
            retry: None,
            focus: Confirm::No,
        }
    }

    /// Say why, as `denial` does.
    #[must_use]
    pub fn because(mut self, denial: Option<Denial>) -> Self {
        self.denial = denial;
        self
    }

    /// Offer `retry` to an administrator, with the cursor on that button.
    #[must_use]
    pub fn offering(mut self, retry: Option<ElevatedRetry>) -> Self {
        self.focus = if retry.is_some() {
            Confirm::Yes
        } else {
            Confirm::No
        };
        self.retry = retry;
        self
    }

    #[must_use]
    pub const fn operation(&self) -> Operation {
        self.operation
    }

    /// Where it failed, and why.
    #[must_use]
    pub const fn failed(&self) -> &FailedEntry {
        &self.failed
    }

    #[must_use]
    pub const fn denial(&self) -> Option<&Denial> {
        self.denial.as_ref()
    }

    #[must_use]
    pub const fn done(&self) -> usize {
        self.done
    }

    #[must_use]
    pub const fn left(&self) -> usize {
        self.left
    }

    #[must_use]
    pub const fn retry(&self) -> Option<&ElevatedRetry> {
        self.retry.as_ref()
    }

    /// Whether the retry button has the cursor.
    #[must_use]
    pub const fn retry_focused(&self) -> bool {
        self.retry.is_some() && self.focus.is_yes()
    }

    /// Move the cursor to the other button, when there are two.
    pub const fn toggle_focus(&mut self) {
        if self.retry.is_some() {
            self.focus = self.focus.toggled();
        }
    }

    /// The retry, taken out of the dialog to be run.
    pub const fn take_retry(&mut self) -> Option<ElevatedRetry> {
        self.retry.take()
    }
}
