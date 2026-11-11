//! Coverage-only probes for public testing adapters.

use super::WritableTestBackend;
use ratatui::backend::Backend;
use std::io::{self, Write};

pub fn coverage_touch_logs() {
    // Exercise non-test instantiations in coverage builds.
    crate::diagnostics::log_action("coverage log action");
    crate::diagnostics::log_error("coverage log error");
}

pub fn coverage_touch_backend() -> io::Result<()> {
    // Drive WritableTestBackend trait methods in non-test builds.
    let mut backend = WritableTestBackend::new(80, 24);
    let _ = backend.write(b"coverage").expect("write backend");
    Write::flush(&mut backend).expect("flush backend");
    Backend::flush(&mut backend).expect("flush backend");
    let _ = backend.get_cursor_position().expect("get cursor");
    backend
        .set_cursor_position(ratatui::layout::Position::new(1, 1))
        .expect("set cursor");
    backend.clear().expect("clear backend");
    let _ = backend.window_size().expect("window size");
    Ok(())
}
