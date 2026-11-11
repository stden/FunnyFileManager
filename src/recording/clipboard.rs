//! Clipboard reads are input data; writes retain the normal host behavior.
use super::{ClipboardReads, Entry, Input, Recorder};
use crate::clipboard::ClipboardBackend;
use std::{cell::RefCell, rc::Rc};

enum Reads {
    Record(Rc<RefCell<Recorder>>),
    Replay(ClipboardReads),
}

struct Clipboard {
    inner: Box<dyn ClipboardBackend>,
    reads: Reads,
}

impl Input {
    pub(crate) fn clipboard(&self, inner: Box<dyn ClipboardBackend>) -> Box<dyn ClipboardBackend> {
        let reads = match self {
            Self::Live(None) => return inner,
            Self::Live(Some(recorder)) => Reads::Record(Rc::clone(recorder)),
            Self::Replay(replay) => Reads::Replay(Rc::clone(&replay.clipboard)),
        };
        Box::new(Clipboard { inner, reads })
    }
}

impl ClipboardBackend for Clipboard {
    fn get_native(&mut self) -> Result<String, String> {
        match &self.reads {
            Reads::Record(recorder) => {
                let result = self.inner.get_native();
                recorder
                    .borrow_mut()
                    .append(&Entry::Clipboard {
                        clipboard: result.clone(),
                    })
                    .map_err(|error| error.to_string())?;
                result
            }
            Reads::Replay(reads) => reads
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err("Recording has no remaining clipboard read".to_owned())),
        }
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        self.inner.set_native(text)
    }

    fn emit_osc52(&mut self, payload: &str) -> Result<(), String> {
        self.inner.emit_osc52(payload)
    }
}
