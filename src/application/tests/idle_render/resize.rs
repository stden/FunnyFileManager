//! A size change must repaint even when no resize event arrives.

use super::*;
use ratatui::{layout::Rect, TerminalOptions, Viewport};

fn redraw_after_silent_resize(viewport: Viewport) -> io::Result<()> {
    let directory = TempDir::new()?;
    let app = app_at(directory.path())?;
    let screen = CountingBackend::new();
    let mut terminal = Terminal::with_options(screen.clone(), TerminalOptions { viewport })?;
    let mut script = [(1, None), (2, Some(quit()))].into_iter();

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || {
            let (draws, event) = script.next().expect("script must quit");
            assert_eq!(screen.draws(), draws, "size change must redraw");
            if event.is_none() {
                screen.resize(Size::new(80, 24));
            }
            Ok(event)
        },
        Arc::new(NoWsl),
    )?;
    assert_eq!(screen.draws(), 2);
    assert!(
        !screen.text().trim().is_empty(),
        "the frame remains visible"
    );
    Ok(())
}

#[test]
fn fullscreen_repaints_after_a_silent_resize() -> io::Result<()> {
    redraw_after_silent_resize(Viewport::Fullscreen)
}

#[test]
fn inline_repaints_after_a_silent_height_change() -> io::Result<()> {
    redraw_after_silent_resize(Viewport::Inline(4))
}

#[test]
fn fixed_viewport_repaints_all_cells_after_a_silent_resize() -> io::Result<()> {
    let directory = TempDir::new()?;
    let app = app_at(directory.path())?;
    let screen = CountingBackend::new();
    let viewport = Viewport::Fixed(Rect::new(0, 0, 40, 8));
    let mut terminal = Terminal::with_options(screen.clone(), TerminalOptions { viewport })?;
    let mut first_frame = Vec::new();
    let mut script = [None, Some(quit())].into_iter();

    run_app_with_wsl_backend(
        &mut terminal,
        app,
        || {
            let event = script.next().expect("script must quit");
            if event.is_none() {
                first_frame = screen.rows()[..8].to_vec();
                screen.resize(Size::new(80, 24));
            } else {
                assert_eq!(&screen.rows()[..8], first_frame);
            }
            Ok(event)
        },
        Arc::new(NoWsl),
    )
}
