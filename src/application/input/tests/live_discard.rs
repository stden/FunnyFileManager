//! Purpose: asking a live keyboard with no terminal behind it to discard keys finds no cancel.

use super::*;

#[test]
fn a_live_recording_without_a_terminal_discards_nothing() -> io::Result<()> {
    let mut live = crate::recording::Input::Live(None);
    let mut input = BufferedInput::new(&mut live);

    assert!(!input.discard_queued()?);
    Ok(())
}
