//! Unit tests for `clipboard`.
//!
//! Everything runs against a recording fake, so no test needs a display server,
//! a terminal, or the developer's real clipboard.

use super::*;
use std::io::Write;

use std::cell::RefCell;
use std::rc::Rc;

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
        self.native_calls.push(text.to_string());
        if self.native_ok {
            return Ok(());
        }
        // Deliberately quote the value we were handed: real platform clipboard
        // layers do format their input into diagnostics, and `copy_secret` is
        // required to strip it back out before the message reaches the UI.
        Err(format!("selection refused for {text:?}"))
    }

    fn emit_osc52(&mut self, payload: &str) -> Result<(), String> {
        self.osc52_calls.push(payload.to_string());
        if self.osc52_ok {
            return Ok(());
        }
        // Same trap on the fallback route: the payload embeds the base64 of the
        // secret, which is just as much a leak as the plaintext.
        Err(format!("terminal write failed for {payload:?}"))
    }
}

/// A `Write` sink whose bytes stay readable after `SystemClipboard` owns it.
#[derive(Clone, Default)]
struct SharedSink(Rc<RefCell<Vec<u8>>>);

impl Write for SharedSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
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

#[test]
fn base64_matches_rfc4648_vectors() {
    let vectors = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];
    for (input, expected) in vectors {
        assert_eq!(
            base64_encode(input.as_bytes()),
            expected,
            "base64({input:?})"
        );
    }
}

#[test]
fn base64_encodes_high_bytes_without_sign_or_truncation_errors() {
    // 0xFF 0xFE 0xFD exercises every high bit; "пароль" is multi-byte UTF-8,
    // which is what a real password field can hold.
    assert_eq!(base64_encode(&[0xFF, 0xFE, 0xFD]), "//79");
    assert_eq!(base64_encode("пароль".as_bytes()), "0L/QsNGA0L7Qu9GM");
}

#[test]
fn system_clipboard_writes_osc52_bytes_to_the_injected_sink() {
    let sink = SharedSink::default();
    // Never call `set_native` here: that would write the developer's real
    // clipboard. Only the terminal route is under test.
    let mut clipboard = SystemClipboard::terminal_only(Box::new(sink.clone()));

    clipboard
        .emit_osc52(&osc52_sequence("foobar"))
        .expect("writing to an in-memory sink cannot fail");

    assert_eq!(
        String::from_utf8(sink.0.borrow().clone()).expect("valid utf-8"),
        "\x1b]52;c;Zm9vYmFy\x07",
        "OSC 52 must go to the caller's writer, not straight to stdout"
    );
}

#[test]
fn redact_with_an_empty_secret_returns_the_message_untouched() {
    // `str::replace("", ..)` would match between every character; the guard
    // must hand the message back as-is instead.
    assert_eq!(redact("native failed: x", ""), "native failed: x");
}

#[test]
fn system_clipboard_reports_a_missing_os_clipboard_instead_of_panicking() {
    let mut clipboard = SystemClipboard::terminal_only(Box::new(Vec::new()));
    let error = clipboard
        .set_native("value")
        .expect_err("native route deliberately absent");
    assert!(error.contains("no OS clipboard available"), "{error}");
    assert!(
        !error.contains("value"),
        "the error must not quote the text"
    );
    assert!(clipboard.get_native().is_err());
}
