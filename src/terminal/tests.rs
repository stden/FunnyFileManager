use super::*;
use crate::WritableTestBackend;
use std::cell::RefCell;
use std::rc::Rc;

mod session;
mod suspension;

#[derive(Clone)]
struct RecordingOps {
    calls: Rc<RefCell<Vec<&'static str>>>,
    fail_on: Option<&'static str>,
    /// What the pretend host supports: `Kitty` unless a test says otherwise.
    keyboard: KeyboardProtocol,
}

impl RecordingOps {
    fn new(fail_on: Option<&'static str>) -> Self {
        Self {
            calls: Rc::new(RefCell::new(Vec::new())),
            fail_on,
            keyboard: KeyboardProtocol::Kitty,
        }
    }

    fn legacy() -> Self {
        Self {
            keyboard: KeyboardProtocol::Legacy,
            ..Self::new(None)
        }
    }

    fn record(&self, operation: &'static str) -> io::Result<()> {
        self.calls.borrow_mut().push(operation);
        if self.fail_on == Some(operation) {
            Err(io::Error::other(operation))
        } else {
            Ok(())
        }
    }
}

impl TerminalOps<WritableTestBackend> for RecordingOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        self.record("enable_raw")
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        self.record("disable_raw")
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("enter_alternate")
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("leave_alternate")
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("enable_mouse")
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("disable_mouse")
    }

    fn clear(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("clear")
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        self.record("show_cursor")
    }

    fn enable_keyboard(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
    ) -> io::Result<KeyboardProtocol> {
        self.record("enable_keyboard").map(|()| self.keyboard)
    }

    fn disable_keyboard(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
    ) -> io::Result<KeyboardProtocol> {
        self.record("disable_keyboard").map(|()| self.keyboard)
    }

    fn wait_for_key(
        &mut self,
        _terminal: &mut Terminal<WritableTestBackend>,
        _prompt: &str,
    ) -> io::Result<()> {
        self.record("wait_for_key")
    }
}

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal")
}
