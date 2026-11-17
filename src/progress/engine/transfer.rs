//! Purpose: The file a copy or a move is sending, as the dialog shows it: where
//! it goes, how much of it is copied, and whether it is done.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::super::{FileProgress, Operation, Transfer};
use super::{update, Shown};

thread_local! {
    static DESTINATION: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(super) fn is_staging() -> bool {
    DESTINATION.with(|current| current.borrow().is_some())
}

/// Use the final destination while a copy writes to a temporary staging name.
pub fn with_destination<T>(destination: &Path, copy: impl FnOnce() -> T) -> T {
    struct Restore(Option<PathBuf>);
    impl Drop for Restore {
        fn drop(&mut self) {
            DESTINATION.with(|current| *current.borrow_mut() = self.0.take());
        }
    }
    let previous = DESTINATION.with(|current| current.replace(Some(destination.to_path_buf())));
    let _restore = Restore(previous);
    copy()
}

/// The file `source` is being sent to `destination`, of `size` bytes.
///
/// A rename copies nothing, and a caller that does not know the size yet passes
/// zero: the copy of the file reports its size once it has opened it.
pub fn sending(source: &Path, destination: &Path, size: u64) {
    DESTINATION.with(|current| {
        let override_path = current.borrow();
        update(|shown| {
            shown.send(
                source,
                override_path.as_deref().unwrap_or(destination),
                size,
            );
        });
    });
}

impl Shown {
    /// Only a copy or a move sends files: a search, a Git pull or a delete has
    /// nothing to name.
    fn send(&mut self, source: &Path, destination: &Path, size: u64) {
        if !matches!(self.operation, Operation::Copy | Operation::Move) {
            return;
        }
        self.set_current(source);
        self.transfer = Some(Transfer {
            destination: destination.to_path_buf(),
            file: FileProgress::Copying { done: 0, size },
        });
    }

    /// `bytes` more of the file being sent are copied.
    pub(super) fn copy_bytes(&mut self, bytes: u64) {
        if let Some(transfer) = self.transfer.as_mut() {
            transfer.file = transfer.file.advanced(bytes);
        }
    }

    /// The file being sent is done, whatever its size.
    pub(super) fn finish_file(&mut self) {
        if let Some(transfer) = self.transfer.as_mut() {
            transfer.file = FileProgress::Done;
        }
    }
}
