//! Purpose: Put a secret (a kdbx password) on the clipboard, by whichever route
//! the environment actually supports, through an injectable backend so tests
//! never touch a real clipboard.
//!
//! Two routes are tried in order:
//!
//! 1. **Native** — the OS clipboard via `arboard`. Works locally and integrates
//!    with every other application.
//! 2. **OSC 52** — an escape sequence the *terminal emulator* interprets, so a
//!    password copied on a remote machine still lands in the local clipboard
//!    over a plain SSH session with no display server.
//!
//! [`copy_secret`] reports which route carried the value, because OSC 52 is
//! **silently ignored** by terminals that disable it (many do, precisely
//! because it can write the clipboard): the write succeeds and nothing arrives.
//! The caller must tell the user which happened instead of promising success.
//!
//! Secrets rule (see `keepass.rs`): the copied text is never logged, and never
//! embedded in a returned error — errors here reach the status bar, so
//! [`copy_secret`] redacts the value out of whatever the backend reported.

/// Placeholder substituted for a secret that a backend echoed back at us.
const REDACTED: &str = "<redacted>";

/// Standard base64 alphabet (RFC 4648) — the one OSC 52 payloads use.
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Which route actually carried the value to a clipboard.
///
/// Worth surfacing to the user: `Native` means the value *is* on the clipboard,
/// while `Osc52` means it was handed to the terminal, which may have dropped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyRoute {
    /// The OS clipboard, via `arboard`.
    Native,
    /// An OSC 52 escape sequence written to the terminal.
    Osc52,
}

/// Sink for clipboard writes.
///
/// Injected rather than called directly so tests can assert the exact bytes
/// without a display server, a terminal, or a real clipboard in the loop.
pub trait ClipboardBackend {
    /// Put `text` on the OS clipboard.
    fn set_native(&mut self, text: &str) -> Result<(), String>;

    /// Write `payload` — a complete, ready-to-emit OSC 52 escape sequence as
    /// built by [`osc52_sequence`] — to the terminal.
    ///
    /// The framing deliberately lives in [`osc52_sequence`] and not in the
    /// implementations, so every backend emits identical bytes and the bytes a
    /// test records are the bytes production writes.
    fn emit_osc52(&mut self, payload: &str) -> Result<(), String>;
}

/// Copy `text`, preferring the native clipboard and falling back to OSC 52.
///
/// Returns the route that succeeded so the caller can be honest in the status
/// line. On failure the error never contains `text` (see [`redact`]).
pub fn copy_secret(backend: &mut dyn ClipboardBackend, text: &str) -> Result<CopyRoute, String> {
    let native_error = match backend.set_native(text) {
        Ok(()) => return Ok(CopyRoute::Native),
        Err(error) => error,
    };
    match backend.emit_osc52(&osc52_sequence(text)) {
        Ok(()) => Ok(CopyRoute::Osc52),
        Err(osc52_error) => Err(redact(
            &format!("clipboard unavailable (native: {native_error}; osc52: {osc52_error})"),
            text,
        )),
    }
}

/// Empty the clipboard by the same route [`copy_secret`] would use.
///
/// Backs the auto-clear timer: a password must not sit on the clipboard after
/// the user pasted it. Setting the empty string is the clear operation for both
/// routes — an OSC 52 sequence with an empty payload clears the selection.
pub fn clear(backend: &mut dyn ClipboardBackend) -> Result<(), String> {
    copy_secret(backend, "").map(|_route| ())
}

/// Build the complete OSC 52 sequence that sets the `c` (clipboard) selection.
///
/// `ESC ] 52 ; c ; <base64> BEL`. BEL (`\x07`) terminates rather than ST
/// (`ESC \`) because every terminal implementing OSC 52 accepts BEL, while some
/// multiplexers mangle ST.
#[must_use]
pub fn osc52_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

/// Encode `input` as standard base64 with `=` padding.
///
/// Hand-rolled rather than adding a dependency: one fixed alphabet, no options,
/// ~20 lines, pinned to the RFC 4648 vectors in the tests.
fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let mut triple = 0_u32;
        for (i, byte) in chunk.iter().enumerate() {
            triple |= u32::from(*byte) << (16 - 8 * i);
        }
        // A 3-byte chunk yields 4 characters; a short final chunk yields
        // `len + 1` characters and pads the rest with '='.
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(
                    BASE64_ALPHABET[((triple >> (18 - 6 * i)) & 0x3F) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Strip `secret` out of a message on its way to the caller.
///
/// A backend's error can quote the value it was handed (platform clipboard
/// layers format their input into diagnostics), and this string is shown in the
/// status bar. Both forms are stripped: the plaintext, and its base64 encoding —
/// an OSC 52 payload is trivially decodable, so leaking it leaks the password.
fn redact(message: &str, secret: &str) -> String {
    // Guard: `str::replace("", ..)` matches between every character and would
    // shred the message. An empty secret is also nothing to hide.
    if secret.is_empty() {
        return message.to_string();
    }
    message
        .replace(secret, REDACTED)
        .replace(&base64_encode(secret.as_bytes()), REDACTED)
}

mod system;
pub use system::SystemClipboard;

#[cfg(test)]
mod tests;
