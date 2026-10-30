//! Unit tests: the system clipboard — OSC 52 bytes reach the injected sink,
//! and a missing OS clipboard is an error, not a panic.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use super::*;

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
