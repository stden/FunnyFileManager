//! Purpose: Record native clipboard attempts while keeping every test away from the real OS.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::clipboard::ClipboardBackend;

#[derive(Default)]
struct State {
    writes: Vec<String>,
    reads: VecDeque<String>,
    write_error: Option<String>,
    osc52_calls: usize,
}

#[derive(Clone, Default)]
pub(super) struct FakeNative(Rc<RefCell<State>>);

impl FakeNative {
    pub(super) fn fail_writes(&self, error: &str) {
        self.0.borrow_mut().write_error = Some(error.to_owned());
    }
    pub(super) fn read_next(&self, text: &str) {
        self.0.borrow_mut().reads.push_back(text.to_owned());
    }
    pub(super) fn writes(&self) -> Vec<String> {
        self.0.borrow().writes.clone()
    }
    pub(super) fn osc52_calls(&self) -> usize {
        self.0.borrow().osc52_calls
    }
}

impl ClipboardBackend for FakeNative {
    fn get_native(&mut self) -> Result<String, String> {
        self.0
            .borrow_mut()
            .reads
            .pop_front()
            .ok_or_else(|| "native unavailable".to_owned())
    }
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        state.writes.push(text.to_owned());
        state.write_error.clone().map_or(Ok(()), Err)
    }
    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        self.0.borrow_mut().osc52_calls += 1;
        Ok(())
    }
}
