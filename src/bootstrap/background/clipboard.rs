//! Purpose: Own zeroizing clipboard text per background session, with optional native integration.

use std::io::{self, Write};
use zeroize::Zeroizing;

use crate::{
    automation::options::Profile,
    clipboard::{ClipboardBackend, SystemClipboard},
};

#[derive(Default)]
enum Native {
    #[default]
    Disabled,
    Backend(Box<dyn ClipboardBackend>),
}

/// Clipboard content is deliberately neither cloneable nor printable with Debug.
#[derive(Default)]
pub(super) struct SessionClipboard {
    native: Native,
    local: Option<Zeroizing<String>>,
}

impl SessionClipboard {
    pub(super) fn for_profile(profile: Profile) -> Self {
        match profile {
            Profile::Isolated => Self::default(),
            Profile::System => Self::with_native(Box::new(SystemClipboard::new(Box::new(
                NoTerminalClipboard,
            )))),
        }
    }

    fn with_native(backend: Box<dyn ClipboardBackend>) -> Self {
        Self {
            native: Native::Backend(backend),
            local: None,
        }
    }
}

impl ClipboardBackend for SessionClipboard {
    fn get_native(&mut self) -> Result<String, String> {
        if let Native::Backend(native) = &mut self.native {
            if let Ok(text) = native.get_native() {
                self.local = Some(Zeroizing::new(text.clone()));
                return Ok(text);
            }
        }
        self.local
            .as_ref()
            .map(|text| text.as_str().to_owned())
            .ok_or_else(|| "session clipboard is empty".to_owned())
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        self.local = Some(Zeroizing::new(text.to_owned()));
        if let Native::Backend(native) = &mut self.native {
            if let Err(error) = native.set_native(text) {
                drop(Zeroizing::new(error));
            }
        }
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("terminal clipboard output is unavailable in background sessions".to_owned())
    }
}

struct NoTerminalClipboard;
impl Write for NoTerminalClipboard {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No terminal clipboard is available",
        ))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
