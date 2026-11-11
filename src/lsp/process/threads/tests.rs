//! Purpose: Stderr sniffing stays bounded and never turns truncated failures into toolchain advice.

use super::*;

const MISSING: &[u8] = b"error: rust-analyzer is not installed for the toolchain 'fixture'";

#[test]
fn crossing_the_sniff_limit_flushes_deferred_sizes_and_bounds_the_prefix() {
    let mut capture = StderrCapture::default();
    capture.push(MISSING);
    assert_eq!(capture.prefix, MISSING);
    assert_eq!(capture.deferred_bytes, MISSING.len());
    assert!(!capture.streamed);
    capture.push(&vec![b'x'; STDERR_SNIFF_LIMIT * 2]);
    assert!(capture.streamed);
    assert_eq!(capture.deferred_bytes, 0);
    assert_eq!(capture.prefix.len(), STDERR_SNIFF_LIMIT);
    let prefix = capture.prefix.clone();
    capture.push(b"later output");
    assert_eq!(capture.prefix, prefix);
    assert_eq!(
        capture.finish(false),
        None,
        "a long stderr is not a short rustup hint"
    );
}

#[test]
fn a_failed_stderr_read_never_reports_an_incomplete_toolchain_hint() {
    let mut complete = StderrCapture::default();
    complete.push(MISSING);
    assert_eq!(complete.finish(false), Some(RUSTUP_RUST_ANALYZER_HINT));
    let mut interrupted = StderrCapture::default();
    interrupted.push(MISSING);
    assert_eq!(interrupted.finish(true), None);
    assert_eq!(StderrCapture::default().finish(true), None);
}
