//! Purpose: the production [`ClipboardBackend`] — the only code that touches
//! the real OS clipboard (via `arboard`) and the real terminal (for OSC 52).
//!
//! Split out of `clipboard.rs` so the coverage policy can exclude it by file
//! name, like `main.rs`/`editor.rs`/`bootstrap.rs`: whether `arboard` can open
//! a clipboard, and whether `set_text` reaches one, depends on the machine the
//! tests run on (headless CI has none), not on this crate's logic. All the
//! logic — routing, base64, redaction — lives in `clipboard.rs` and is fully
//! measured; this file is deliberately nothing but glue.

use std::io::Write;

use super::ClipboardBackend;

/// Production backend: `arboard` for the native route, a caller-supplied
/// writer for OSC 52.
pub struct SystemClipboard {
    /// `None` when no OS clipboard could be opened (headless box, no display).
    /// Held for the lifetime of the struct because on X11/Wayland the clipboard
    /// contents survive only as long as the owning connection does.
    native: Option<arboard::Clipboard>,
    /// Where OSC 52 bytes go. The TUI owns stdout, and a stray `print!` while a
    /// frame is being drawn corrupts the screen — so the caller injects the
    /// writer it already controls instead of this module grabbing stdout.
    sink: Box<dyn Write>,
}

impl SystemClipboard {
    /// Open the OS clipboard (best effort) and route OSC 52 bytes to `sink`.
    ///
    /// A clipboard that fails to open is not an error here: it degrades to the
    /// OSC 52 route, which is exactly the remote-session case.
    #[must_use]
    pub fn new(sink: Box<dyn Write>) -> Self {
        Self {
            native: arboard::Clipboard::new().ok(),
            sink,
        }
    }

    /// Convenience for a caller that is not mid-frame and owns stdout directly.
    #[must_use]
    pub fn with_stdout() -> Self {
        Self::new(Box::new(std::io::stdout()))
    }
}

impl ClipboardBackend for SystemClipboard {
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        let clipboard = self
            .native
            .as_mut()
            .ok_or_else(|| "no OS clipboard available".to_string())?;
        clipboard.set_text(text).map_err(|error| error.to_string())
    }

    fn emit_osc52(&mut self, payload: &str) -> Result<(), String> {
        // Flush explicitly: a buffered escape sequence that only reaches the
        // terminal at the next draw would arrive after the status line already
        // claimed the copy succeeded.
        self.sink
            .write_all(payload.as_bytes())
            .and_then(|()| self.sink.flush())
            .map_err(|error| error.to_string())
    }
}
