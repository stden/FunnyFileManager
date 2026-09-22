//! Unit tests: the [`WritableTestBackend`] delegations that no rendering test
//! reaches, but ratatui calls on a real terminal.

use super::WritableTestBackend;

use ratatui::backend::{Backend, ClearType};
use ratatui::buffer::Cell;
use ratatui::layout::Position;

use std::io;

/// Fill every cell of a `width` x `height` backend with `symbol`.
fn fill(backend: &mut WritableTestBackend, width: u16, height: u16, symbol: &'static str) {
    let cell = Cell::new(symbol);
    let positions = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
    backend
        .draw(positions.map(|(x, y)| (x, y, &cell)))
        .expect("draw");
}

fn symbol_at(backend: &WritableTestBackend, x: u16, y: u16) -> String {
    backend
        .inner
        .buffer()
        .cell((x, y))
        .expect("cell in bounds")
        .symbol()
        .to_string()
}

/// `clear_region` must reach the wrapped backend and honour the clear type —
/// ratatui uses it for `insert_before`, so a no-op or an always-clear-all
/// delegation corrupts the screen.
#[test]
fn clear_region_clears_only_the_cursor_line() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(2, 2);
    fill(&mut backend, 2, 2, "x");
    assert_eq!(symbol_at(&backend, 0, 1), "x", "fill should have worked");

    backend.set_cursor_position(Position::new(0, 1))?;
    backend.clear_region(ClearType::CurrentLine)?;

    assert_eq!(symbol_at(&backend, 0, 1), " ");
    assert_eq!(symbol_at(&backend, 1, 1), " ");
    assert_eq!(symbol_at(&backend, 0, 0), "x", "row 0 must be untouched");
    Ok(())
}

/// The public `buffer()` accessor must delegate to the same buffer `draw`
/// wrote into. Every rendering test elsewhere reaches it indirectly through
/// `Terminal::backend()`; this pins the wrapper's own delegation directly,
/// rather than through `.inner.buffer()` as `symbol_at` above does.
#[test]
fn buffer_exposes_the_frame_the_backend_actually_holds() {
    let mut backend = WritableTestBackend::new(2, 2);
    fill(&mut backend, 2, 2, "z");

    assert_eq!(
        backend
            .buffer()
            .cell((0, 0))
            .expect("cell in bounds")
            .symbol(),
        "z"
    );
    assert_eq!(backend.buffer().area.width, 2);
    assert_eq!(backend.buffer().area.height, 2);
}

/// The reported pixel size is what a terminal query would answer; ratatui
/// widgets that scale images divide by it.
#[test]
fn window_size_reports_cell_pixels_for_the_backend_size() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);

    let window = backend.window_size()?;

    assert_eq!(window.columns_rows.width, 80);
    assert_eq!(window.columns_rows.height, 24);
    assert_eq!(window.pixels.width, 80 * 8);
    assert_eq!(window.pixels.height, 24 * 16);
    Ok(())
}

mod root_cases;
