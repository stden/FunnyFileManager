//! Purpose: the kitty keyboard protocol — pushed where the host terminal
//! answers for it, popped on every way out — and what a key means without it.
//!
//! Without it a terminal sends keys as VT bytes, and some combinations share
//! a byte: Ctrl+\ and Ctrl+4 are both 0x1C, Ctrl+] and Ctrl+5 are 0x1D,
//! Ctrl+I is Tab, Ctrl+M is Enter, and xterm's Ctrl/Shift/Alt+F3
//! (`CSI 1;5R`) is read by crossterm as a cursor-position report. With it,
//! every combination arrives as its own sequence (F3 becomes `CSI 13;5~`).

use std::io::{self, Write};

use crossterm::event::{
    Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
#[cfg(unix)]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;

/// Which keyboard protocol this program has asked the host terminal for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardProtocol {
    /// Nothing pushed. On Unix keys arrive as VT bytes, with the collisions
    /// listed above; the Windows console reports key codes and has none.
    Legacy,
    /// The kitty protocol, pushed by this program; it must be popped before
    /// the alternate screen is left, because each screen has its own stack.
    Kitty,
}

#[cfg(unix)]
impl KeyboardProtocol {
    /// The key the user pressed, as far as this protocol can tell.
    ///
    /// Without the protocol Ctrl+\ and Ctrl+4 are one byte, 0x1C, which
    /// crossterm reports as Ctrl+4. It is taken as Ctrl+\ — Go to root is
    /// the key people press, and Wide view stays in the Left / Right menu.
    /// Ctrl+] and Ctrl+5 share 0x1D the same way, but no screen gives both a
    /// meaning, so that one is left to the editor keymap.
    #[must_use]
    pub(crate) fn decode(self, key: KeyEvent) -> KeyEvent {
        let is_file_separator = self == Self::Legacy
            && key.code == KeyCode::Char('4')
            && key.modifiers.contains(KeyModifiers::CONTROL);
        if is_file_separator {
            KeyEvent {
                code: KeyCode::Char('\\'),
                ..key
            }
        } else {
            key
        }
    }
}

/// An input event as the user typed it on this session's terminal.
#[cfg(unix)]
pub fn decode_input(event: Event) -> Event {
    match event {
        Event::Key(key) => Event::Key(known_protocol().decode(key)),
        other => other,
    }
}

/// The Windows console reports key codes, and no two keys share one.
#[cfg(windows)]
pub fn decode_input(event: Event) -> Event {
    event
}

/// Escape codes for every modified key, and the character a shifted key
/// types — so Alt+Shift+= still arrives as Alt+'+', as it does without the
/// protocol, rather than as Alt+Shift+'='.
const FLAGS: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    .union(KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS);

/// The host's answer, asked once per process: the query is a round trip that
/// can wait for a slow terminal, and a resumed session must not ask again.
#[cfg(unix)]
static HOST: std::sync::OnceLock<KeyboardProtocol> = std::sync::OnceLock::new();

/// The protocol the host terminal supports, asking it the first time.
///
/// A terminal that does not answer the query in crossterm's two seconds, or
/// cannot be asked at all, is treated as legacy.
#[cfg(unix)]
pub(super) fn host_protocol() -> KeyboardProtocol {
    *HOST.get_or_init(
        || match crossterm::terminal::supports_keyboard_enhancement() {
            Ok(true) => KeyboardProtocol::Kitty,
            Ok(false) | Err(_) => KeyboardProtocol::Legacy,
        },
    )
}

/// The Windows console reads key codes through the console API, which never
/// confuses two keys, and crossterm cannot push the protocol there.
#[cfg(windows)]
pub(super) const fn host_protocol() -> KeyboardProtocol {
    KeyboardProtocol::Legacy
}

/// The protocol a session set up, without asking the terminal anything:
/// legacy until [`host_protocol`] has had its answer.
#[cfg(unix)]
pub(super) fn known_protocol() -> KeyboardProtocol {
    HOST.get().copied().unwrap_or(KeyboardProtocol::Legacy)
}

#[cfg(windows)]
pub(super) const fn known_protocol() -> KeyboardProtocol {
    KeyboardProtocol::Legacy
}

/// Push the protocol when the host supports it; the answer is the protocol
/// now in force.
pub(super) fn push(out: &mut impl Write, host: KeyboardProtocol) -> io::Result<KeyboardProtocol> {
    if host == KeyboardProtocol::Kitty {
        execute!(out, PushKeyboardEnhancementFlags(FLAGS))?;
    }
    Ok(host)
}

/// Pop what [`push`] pushed; the answer is the protocol to push again on
/// resume — `Kitty` when flags were popped, `Legacy` when there were none.
pub(super) fn pop(out: &mut impl Write, host: KeyboardProtocol) -> io::Result<KeyboardProtocol> {
    if host == KeyboardProtocol::Kitty {
        execute!(out, PopKeyboardEnhancementFlags)?;
    }
    Ok(host)
}

#[cfg(test)]
mod tests;
