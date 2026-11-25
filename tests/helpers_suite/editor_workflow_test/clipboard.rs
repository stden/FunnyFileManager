//! Purpose: An in-memory clipboard backend that records copies and reads and can fail on demand.

use super::*;

#[derive(Default)]
pub struct ClipboardState {
    pub text: String,
    pub copied: Vec<String>,
    pub reads: usize,
    pub fail_read: bool,
}

#[derive(Clone, Default)]
pub struct Clipboard(pub Rc<RefCell<ClipboardState>>);

impl ClipboardBackend for Clipboard {
    fn get_native(&mut self) -> Result<String, String> {
        let mut state = self.0.borrow_mut();
        state.reads += 1;
        if state.fail_read {
            Err("test clipboard unavailable".to_owned())
        } else {
            Ok(state.text.clone())
        }
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        let mut state = self.0.borrow_mut();
        text.clone_into(&mut state.text);
        state.copied.push(text.to_owned());
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("test has no host terminal".to_owned())
    }
}
