//! Purpose: `CrosstermOps` pinned to what each step writes: the escape sequences and the backend calls, and a failure's own message.

use super::*;

/// The alternate screen is what keeps the user's scrollback intact, so the two
/// halves must emit the matching pair and not each other's sequence.
#[test]
fn crossterm_ops_switch_the_alternate_screen_both_ways() {
    let mut terminal = spy_terminal();
    let mut ops = CrosstermOps;

    ops.enter_alternate(&mut terminal)
        .expect("enter the alternate screen");
    assert_eq!(take_written(&mut terminal), "\u{1b}[?1049h");

    ops.leave_alternate(&mut terminal)
        .expect("leave the alternate screen");
    assert_eq!(take_written(&mut terminal), "\u{1b}[?1049l");
}

/// Mouse capture must be switched off again on the way out — a terminal left in
/// SGR mouse mode reports clicks as garbage input to whatever runs next.
#[test]
fn crossterm_ops_toggle_mouse_capture_both_ways() {
    // What each half hands the backend. Mouse capture is the one operation
    // crossterm reports as *not* ANSI-supported on Windows: there it drives the
    // console API instead, so the contract flips to "writes nothing" — a stray
    // `?1000h` would land in the alternate screen as visible garbage.
    #[cfg(not(windows))]
    let (on, off) = (
        "\u{1b}[?1000h\u{1b}[?1002h\u{1b}[?1003h\u{1b}[?1015h\u{1b}[?1006h",
        "\u{1b}[?1006l\u{1b}[?1015l\u{1b}[?1003l\u{1b}[?1002l\u{1b}[?1000l",
    );
    #[cfg(windows)]
    let (on, off) = ("", "");

    let _lock = crate::test_support::editor_env_lock();
    let mut terminal = spy_terminal();
    let mut ops = CrosstermOps;

    // Enable first either way: the console-API path refuses to disable capture
    // it never enabled ("Initial console modes not set").
    ops.enable_mouse(&mut terminal)
        .expect("enable mouse capture");
    assert_eq!(take_written(&mut terminal), on);

    ops.disable_mouse(&mut terminal)
        .expect("disable mouse capture");
    assert_eq!(take_written(&mut terminal), off);
}

/// `clear` must wipe the screen and `show_cursor` must give the cursor back —
/// the two operations that run after the alternate screen is gone, so a no-op
/// here leaves the user staring at a blinking-cursor-less shell.
#[test]
fn crossterm_ops_clear_and_show_cursor_reach_the_backend() {
    let mut terminal = spy_terminal();
    let mut ops = CrosstermOps;

    ops.clear(&mut terminal).expect("clear the screen");
    assert!(
        terminal.backend().calls.contains(&"clear_region"),
        "clear never reached the backend: {:?}",
        terminal.backend().calls
    );

    terminal.backend_mut().calls.clear();
    ops.show_cursor(&mut terminal).expect("show the cursor");
    assert_eq!(terminal.backend().calls, ["show_cursor"]);
}

/// Both calls go through ratatui, whose error type is not `io::Error` for every
/// backend. The conversion must carry the backend's own message: it is the only
/// clue the user gets about why the terminal could not be restored.
#[test]
fn crossterm_ops_keep_the_backend_message_when_the_terminal_fails() {
    let mut terminal = spy_terminal();
    terminal.backend_mut().fail = true;
    let mut ops = CrosstermOps;

    let clear_error = ops
        .clear(&mut terminal)
        .expect_err("a failing backend must fail the clear");
    assert_eq!(clear_error.kind(), io::ErrorKind::Other);
    assert!(
        clear_error.to_string().contains(SPY_FAILURE),
        "the backend's message was lost: {clear_error}"
    );

    let cursor_error = ops
        .show_cursor(&mut terminal)
        .expect_err("a failing backend must fail show_cursor");
    assert!(
        cursor_error.to_string().contains(SPY_FAILURE),
        "the backend's message was lost: {cursor_error}"
    );
}

/// `disable_raw` is the one `CrosstermOps` call with no backend to record it.
/// Crossterm makes it a no-op when raw mode was never entered, so the outcome
/// to assert is "succeeded, and raw mode is off". That is enough to catch the
/// realistic slip — calling `enable_raw_mode` here — on both kinds of runner:
/// it errors without a tty, and it reports raw mode *on* with one.
#[test]
fn crossterm_ops_disable_raw_leaves_raw_mode_off() {
    // Serialised against the external-editor test, the only other test in this
    // binary that can switch the process-wide raw mode on.
    let _lock = crate::test_support::editor_env_lock();
    let mut ops = CrosstermOps;

    TerminalOps::<SpyBackend>::disable_raw(&mut ops).expect("disable raw mode");

    assert!(
        !crossterm::terminal::is_raw_mode_enabled().expect("query raw mode"),
        "raw mode is still on after disable_raw"
    );
}
