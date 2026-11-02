//! Purpose: The test seam a wipe reports its moments to: the zeros are the
//! point of the feature and the next step erases them, so this is the only
//! place they can be read back.

use std::path::Path;

/// The moments a wipe can be observed at, for the test seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The zeros are on disk and the file still has its length.
    Overwritten,
    /// The entry has its temporary name and is about to be removed.
    Renamed,
}

/// What a test installs to watch a wipe happen.
#[cfg(any(test, tarpaulin))]
pub type Observer = Box<dyn FnMut(Stage, &Path)>;

#[cfg(any(test, tarpaulin))]
thread_local! {
    /// Test seam: called at each `Stage` with the path as it is at that moment.
    /// The zeros are the point of the feature and the next step erases them,
    /// so this is the only place they can be read back.
    pub static WITNESS: std::cell::RefCell<Option<Observer>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, tarpaulin))]
pub(super) fn witness(stage: Stage, path: &Path) {
    WITNESS.with(|witness| {
        if let Some(observe) = witness.borrow_mut().as_mut() {
            observe(stage, path);
        }
    });
}

#[cfg(not(any(test, tarpaulin)))]
pub(super) const fn witness(_stage: Stage, _path: &Path) {}
