//! Purpose: Session clipboard behavior is deterministic without an OS clipboard or terminal.

use super::*;
use crate::clipboard::{clear, copy_secret, CopyRoute};

mod native;
use native::FakeNative;

#[test]
fn isolated_clipboard_distinguishes_unset_unicode_and_cleared_values() {
    let mut clipboard = SessionClipboard::for_profile(Profile::Isolated);
    assert!(clipboard.get_native().is_err());
    assert!(clipboard.set_native("a界🙂").is_ok());
    assert_eq!(clipboard.get_native().unwrap(), "a界🙂");
    clear(&mut clipboard).unwrap();
    assert_eq!(clipboard.get_native().unwrap(), "");
}

#[test]
fn two_session_clipboards_never_share_cached_text() {
    let mut first = SessionClipboard::for_profile(Profile::Isolated);
    let mut second = SessionClipboard::for_profile(Profile::Isolated);
    first.set_native("first").unwrap();
    assert!(second.get_native().is_err());
    second.set_native("second").unwrap();
    assert_eq!(first.get_native().unwrap(), "first");
    assert_eq!(second.get_native().unwrap(), "second");
}

#[test]
fn native_write_failure_still_copies_locally_without_exposing_the_error() {
    let native = FakeNative::default();
    native.fail_writes("backend error contains SECRET");
    let mut clipboard = SessionClipboard::with_native(Box::new(native.clone()));
    assert_eq!(
        copy_secret(&mut clipboard, "SECRET").unwrap(),
        CopyRoute::Native
    );
    assert_eq!(clipboard.get_native().unwrap(), "SECRET");
    assert_eq!(native.writes(), ["SECRET"]);
    assert_eq!(native.osc52_calls(), 0);
    clear(&mut clipboard).unwrap();
    assert_eq!(clipboard.get_native().unwrap(), "");
}

#[test]
fn native_reads_take_priority_and_refresh_the_zeroizing_fallback() {
    let native = FakeNative::default();
    let mut clipboard = SessionClipboard::with_native(Box::new(native.clone()));
    clipboard.set_native("cached").unwrap();
    native.read_next("native-value");
    assert_eq!(clipboard.get_native().unwrap(), "native-value");
    assert_eq!(clipboard.get_native().unwrap(), "native-value");
    assert_eq!(native.writes(), ["cached"]);
}

#[test]
fn a_native_empty_value_overrides_stale_cached_text() {
    let native = FakeNative::default();
    let mut clipboard = SessionClipboard::with_native(Box::new(native.clone()));
    clipboard.set_native("old").unwrap();
    native.read_next("");
    assert_eq!(clipboard.get_native().unwrap(), "");
    assert_eq!(clipboard.get_native().unwrap(), "");
}

#[test]
fn clearing_for_secret_retention_clears_both_native_and_cached_text() {
    let native = FakeNative::default();
    let mut clipboard = SessionClipboard::with_native(Box::new(native.clone()));
    clipboard.set_native("retained-secret").unwrap();
    clear(&mut clipboard).unwrap();
    assert_eq!(clipboard.get_native().unwrap(), "");
    assert_eq!(native.writes(), ["retained-secret", ""]);
}

#[test]
fn osc52_is_rejected_without_forwarding_or_echoing_the_payload() {
    let native = FakeNative::default();
    let mut clipboard = SessionClipboard::with_native(Box::new(native.clone()));
    let error = clipboard.emit_osc52("PAYLOAD-MUST-NOT-LEAK").unwrap_err();
    assert!(!error.contains("PAYLOAD-MUST-NOT-LEAK"));
    assert_eq!(native.osc52_calls(), 0);
    assert_eq!(native.writes(), Vec::<String>::new());
}

#[test]
fn terminal_sink_rejects_writes_without_emitting_any_bytes() {
    let mut sink = NoTerminalClipboard;
    assert_eq!(
        sink.write(b"secret").unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(sink.flush().is_ok());
}
