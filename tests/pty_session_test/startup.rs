//! Purpose: An ffm that ends before its first frame fails the fixture at once,
//! not when the whole start-up wait runs out.

use super::*;

#[test]
#[should_panic(expected = "ffm ended before its first frame")]
fn interactive_startup_rejects_a_process_that_ends_without_a_frame() {
    let path = std::env::var_os("PATH").unwrap_or_default();
    Ffm::start_in(TempDir::new().expect("sandbox"), &path, &["--invalid-flag"]);
}
