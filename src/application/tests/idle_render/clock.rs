//! Purpose: an idle screen repaints its clock once when the minute turns over, and not on every poll.

use super::*;

const TWENTY_ONE_PAST_ONE: u64 = 13 * 3_600 + 21 * 60;
const TWENTY_TWO_PAST_ONE: u64 = TWENTY_ONE_PAST_ONE + 60;

/// The viewer's status line reads the clock at draw time, and an idle screen
/// draws nothing, so it used to show the minute of the last key for good.
/// The minute turning over repaints once — once, not on every poll after it.
#[test]
fn an_idle_viewer_repaints_its_clock_once_when_the_minute_turns_over() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut app = app_at(directory.path())?;
    app.set_overlay(Overlay::Viewer(Viewer::report("Report", vec![])));
    crate::clock::set_test_time(Some(TWENTY_ONE_PAST_ONE));
    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut reads = 0;

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || {
            reads += 1;
            Ok(clock_poll(reads, &screen))
        },
        Arc::new(NoWsl),
    )?;

    assert_eq!(reads, 5);
    Ok(())
}

/// One read of the clock test: what the screen must show by now, and what
/// happens next.
fn clock_poll(read: usize, screen: &CountingBackend) -> Option<Event> {
    let draws = screen.draws();
    match read {
        1 => assert_eq!(draws, 1, "the first frame"),
        2 => {
            assert_eq!(draws, 1, "the same minute must not repaint");
            assert!(screen.text().contains(" 1:21 PM"), "{}", screen.text());
            crate::clock::set_test_time(Some(TWENTY_TWO_PAST_ONE));
        }
        3 => {
            assert_eq!(draws, 2, "the minute turned over on an idle screen");
            assert!(screen.text().contains(" 1:22 PM"), "{}", screen.text());
        }
        4 => {
            assert_eq!(draws, 2, "one frame per minute, not one per poll");
            return Some(key(KeyCode::Esc));
        }
        _ => return Some(quit()),
    }
    None
}
