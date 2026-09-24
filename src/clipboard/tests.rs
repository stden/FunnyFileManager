//! Unit tests for `clipboard`.
//!
//! Everything runs against a recording fake, so no test needs a display server,
//! a terminal, or the developer's real clipboard. Base64 and redaction are in
//! `tests/tests_encoding.rs`, the system clipboard's terminal route in
//! `tests/tests_system.rs`.

use super::*;

mod tests_encoding;
mod tests_system;

/// A secret long enough that an accidental substring match cannot be luck.
const SECRET: &str = "correct horse battery staple";

/// Records what production asked for, and can fail either route on demand.
#[derive(Default)]
struct FakeBackend {
    native_ok: bool,
    osc52_ok: bool,
    native_calls: Vec<String>,
    osc52_calls: Vec<String>,
}

impl FakeBackend {
    const fn new(native_ok: bool, osc52_ok: bool) -> Self {
        Self {
            native_ok,
            osc52_ok,
            native_calls: Vec::new(),
            osc52_calls: Vec::new(),
        }
    }
}

impl ClipboardBackend for FakeBackend {
    fn set_native(&mut self, text: &str) -> Result<(), String> {
        self.native_calls.push(text.to_owned());
        if self.native_ok {
            return Ok(());
        }
        // Deliberately quote the value we were handed: real platform clipboard
        // layers do format their input into diagnostics, and `copy_secret` is
        // required to strip it back out before the message reaches the UI.
        Err(format!("selection refused for {text:?}"))
    }

    fn emit_osc52(&mut self, payload: &str) -> Result<(), String> {
        self.osc52_calls.push(payload.to_owned());
        if self.osc52_ok {
            return Ok(());
        }
        // Same trap on the fallback route: the payload embeds the base64 of the
        // secret, which is just as much a leak as the plaintext.
        Err(format!("terminal write failed for {payload:?}"))
    }
}

#[test]
fn native_success_reports_native_route_and_skips_osc52() {
    let mut backend = FakeBackend::new(true, true);

    let route = copy_secret(&mut backend, SECRET).expect("native route should carry the value");

    assert_eq!(route, CopyRoute::Native);
    assert_eq!(backend.native_calls, [SECRET]);
    assert!(
        backend.osc52_calls.is_empty(),
        "OSC 52 must not be emitted once the native clipboard accepted the value: {:?}",
        backend.osc52_calls
    );
}

#[test]
fn native_failure_falls_back_to_osc52_with_exact_escape_bytes() {
    let mut backend = FakeBackend::new(false, true);

    let route = copy_secret(&mut backend, "foobar").expect("OSC 52 fallback should carry it");

    assert_eq!(route, CopyRoute::Osc52);
    assert_eq!(backend.native_calls, ["foobar"], "native route tried first");
    assert_eq!(backend.osc52_calls, ["\x1b]52;c;Zm9vYmFy\x07"]);
}

#[test]
fn both_routes_failing_returns_an_error_that_leaks_neither_form_of_the_secret() {
    let mut backend = FakeBackend::new(false, false);

    let error = copy_secret(&mut backend, SECRET).expect_err("no route can carry the value");

    assert!(
        !error.contains(SECRET),
        "returned error leaked the plaintext secret: {error}"
    );
    assert!(
        !error.contains(&base64_encode(SECRET.as_bytes())),
        "returned error leaked the base64 of the secret: {error}"
    );
    assert!(
        error.contains("clipboard unavailable"),
        "error should say what went wrong: {error}"
    );
    assert_eq!(backend.native_calls.len(), 1, "native route was attempted");
    assert_eq!(backend.osc52_calls.len(), 1, "OSC 52 route was attempted");
}

#[test]
fn clear_empties_the_native_clipboard_when_it_is_available() {
    let mut backend = FakeBackend::new(true, true);

    clear(&mut backend).expect("clear should succeed on the native route");

    assert_eq!(backend.native_calls, [""], "clear writes the empty value");
    assert!(
        backend.osc52_calls.is_empty(),
        "no OSC 52 needed once native cleared"
    );
}

#[test]
fn clear_empties_via_osc52_when_native_is_unavailable() {
    let mut backend = FakeBackend::new(false, true);

    clear(&mut backend).expect("clear should fall back to OSC 52");

    assert_eq!(backend.native_calls, [""]);
    assert_eq!(
        backend.osc52_calls,
        ["\x1b]52;c;\x07"],
        "an empty OSC 52 payload is the clear operation"
    );
}
