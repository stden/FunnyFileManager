//! Purpose: RAII guards that enter and restore the terminal session, and suspend it around an external program.

use std::io;

mod session;
mod suspension;

pub use session::TerminalSession;
pub use suspension::TerminalSuspension;

fn retain_first_error(first_error: &mut Option<io::Error>, result: io::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => {
            if first_error.is_none() {
                *first_error = Some(error);
            }
            false
        }
    }
}

fn finish_cleanup(first_error: Option<io::Error>) -> io::Result<()> {
    first_error.map_or(Ok(()), Err)
}
