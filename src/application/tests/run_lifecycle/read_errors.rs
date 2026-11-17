//! Purpose: read errors in the event loop — a lost terminal ends the session cleanly, any other read error is still fatal.

use super::*;

/// Losing the terminal is the end of a session, not a failure of one: the loop
/// leaves the way F10 leaves, so the caller still gets to write the histories.
/// Before this, crossterm's reader trapped the process in a loop that read
/// zero bytes forever and burned a core until something killed it.
#[test]
fn a_lost_terminal_ends_the_loop_cleanly() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;
    let reads = std::cell::Cell::new(0_u32);

    run_loop(
        &mut terminal,
        &mut app,
        || {
            reads.set(reads.get() + 1);
            match reads.get() {
                1 => Ok(Some(Event::Key(key_event(KeyCode::Down)))),
                2 => Err(crate::recording::TerminalLost::error()),
                _ => panic!("the loop must not read past a lost terminal"),
            }
        },
        Arc::new(crate::wsl::SystemWslBackend),
    )?;

    assert_eq!(reads.get(), 2, "the loop read on after the hang-up");
    assert_eq!(
        app.lifecycle,
        Lifecycle::Quitting,
        "the session ended, so it must not still look running"
    );
    Ok(())
}

/// Only a lost terminal leaves quietly. Every other read failure still aborts,
/// so the sentinel the replay tests end their scripts with keeps its meaning.
#[test]
fn other_read_errors_are_still_fatal() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_path_buf())?;
    app.right_panel = Panel::new(temp.path().to_path_buf())?;

    let mut terminal = Terminal::new(WritableTestBackend::new(40, 8))?;

    let error = run_loop(
        &mut terminal,
        &mut app,
        || Err(io::Error::new(io::ErrorKind::UnexpectedEof, "done")),
        Arc::new(crate::wsl::SystemWslBackend),
    )
    .expect_err("a read failure that is not a hang-up must stay fatal");

    assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    assert!(
        app.lifecycle.is_running(),
        "a failure is not a quit: the app must not be marked as having quit"
    );
    Ok(())
}
