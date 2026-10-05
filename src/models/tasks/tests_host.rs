//! Purpose: Tests for the host slot: empty until installed, shared by clones,
//! and minting a session no earlier opening had.

use std::sync::Arc;

use super::{
    ControlError, PriorityChange, ProcessHost, ProcessKey, ProcessReader, SampleError, Signal,
    TaskHostSlot, TaskList,
};

/// A host with nothing behind it.
struct Nowhere;

impl ProcessHost for Nowhere {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Err(SampleError::Unsupported)
    }

    fn signal(&self, _: ProcessKey, _: Signal) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }

    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Err(ControlError::Unsupported)
    }
}

#[test]
fn a_new_slot_has_no_host_until_one_is_installed() {
    let mut slot = TaskHostSlot::default();
    assert!(slot.host().is_none());
    slot.install(Arc::new(Nowhere));
    let host = slot.host().expect("installed");
    assert_eq!(host.reader().err(), Some(SampleError::Unsupported));
}

#[test]
fn a_clone_shares_the_installed_host() {
    let mut slot = TaskHostSlot::default();
    slot.install(Arc::new(Nowhere));
    let copy = slot.clone();
    let (first, second) = (slot.host().expect("installed"), copy.host().expect("kept"));
    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn every_opening_gets_a_session_of_its_own() {
    let mut slot = TaskHostSlot::default();
    let first = slot.next_session();
    let second = slot.next_session();
    assert_ne!(first, second);
    assert_ne!(
        first,
        TaskList::default().session(),
        "session 0 is never minted"
    );
}
