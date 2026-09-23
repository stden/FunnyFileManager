//! Purpose: Provides the tiny binary entrypoint that starts the library application.
//!
use std::process::ExitCode;

#[expect(
    clippy::print_stderr,
    reason = "the binary's only error channel: `run` has already restored the terminal"
)]
fn main() -> ExitCode {
    // A failed start is a diagnosis, not a crash: one readable line on stderr
    // and a non-zero status, never a panic carrying a Debug-formatted io::Error.
    if let Err(error) = funny_file_manager::run() {
        eprintln!("ffm: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
