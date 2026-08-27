//! Purpose: the real-terminal entry point — the only code path that touches
//! the host TTY and the on-disk history.
//!
//! Split out of `lib.rs` so the coverage policy can exclude it by file name,
//! the same way it excludes `main.rs` and `editor.rs`: nothing here can run
//! without a real terminal, and everything it calls is covered where it lives
//! (`run_app*` in `lib.rs`, the session guards in `terminal.rs`).

use std::error::Error;
use std::io;

use ratatui::{backend::CrosstermBackend, Terminal};

use crate::models::App;

pub fn run() -> Result<(), Box<dyn Error>> {
    crate::run_with(main_impl)
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    let mut app = App::new()?;
    crate::log_action("App initialized successfully");

    // Load the on-disk command history HERE, at the real production entry point,
    // and nowhere else. App::new() is deliberately pure (session-only history)
    // so that no test — unit or integration — can pollute the user's real
    // ~/.local/share/ffm/history simply by constructing an App. This one line is
    // the sole path that resolves a real on-disk history file; it is unreachable
    // from any test binary because this whole file is excluded from them. The
    // behaviour of History::load() itself stays covered by src/history/tests.rs.
    app.history = crate::history::History::load();

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // `panic = "abort"` skips Drop. The hook still gets one best-effort chance
    // to restore the host terminal before the process terminates. Install it
    // before setup mutates terminal state because setup itself may panic.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        crate::terminal::restore_after_abort();
        previous_hook(info);
    }));

    let mut terminal_ops = crate::terminal::CrosstermOps;
    let mut session = crate::terminal::TerminalSession::enter(&mut terminal, &mut terminal_ops)?;

    let run_result = crate::run_app(session.terminal_mut(), app);
    let cleanup_result = session.finish();

    if let Err(ref err) = run_result {
        eprintln!("Error: {err:?}");
    }

    crate::terminal::merge_run_and_cleanup(run_result, cleanup_result)
        .map_err(std::convert::Into::into)
}
