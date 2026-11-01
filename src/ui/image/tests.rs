//! Rendering remains responsive while images are prepared in a worker.

use super::*;
use ratatui::{backend::TestBackend, style::Color, Terminal};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

fn image(width: u32, height: u32, pixels: &[u8]) -> DecodedImage {
    DecodedImage {
        width,
        height,
        format: "PNG",
        size: 10,
        pixels: Arc::from(pixels),
    }
}

fn draw_image(terminal: &mut Terminal<TestBackend>, area: Rect, image: &DecodedImage) {
    terminal
        .draw(|frame| render(frame, area, image, 1, (0, 0)))
        .expect("draw");
}

fn wait_for_protocol() {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if pump() {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("image preparation did not finish");
}

fn row(terminal: &Terminal<TestBackend>, row: u16) -> String {
    (0..terminal.size().expect("size").width)
        .map(|column| terminal.backend().buffer()[(column, row)].symbol())
        .collect()
}

#[test]
fn fallback_shows_preparing_then_renders_color_without_blocking() {
    set_mode(Mode::Halfblocks);
    let red = image(1, 1, &[255, 0, 0, 255]);
    let area = Rect::new(0, 0, 30, 4);
    let mut terminal = Terminal::new(TestBackend::new(30, 4)).expect("terminal");
    draw_image(&mut terminal, area, &red);
    assert!(row(&terminal, 0).contains("Preparing image"));
    wait_for_protocol();
    draw_image(&mut terminal, area, &red);
    let cell = &terminal.backend().buffer()[(0, 0)];
    assert!(cell.fg == Color::Rgb(255, 0, 0) || cell.bg == Color::Rgb(255, 0, 0));
    clear();
}

#[test]
fn invalid_pixel_buffer_reports_failure_instead_of_remaining_pending() {
    let invalid = image(2, 2, &[1, 2, 3]);
    let area = Rect::new(0, 0, 72, 3);
    let mut terminal = Terminal::new(TestBackend::new(72, 3)).expect("terminal");
    draw_image(&mut terminal, area, &invalid);
    wait_for_protocol();
    draw_image(&mut terminal, area, &invalid);
    let text = row(&terminal, 0);
    assert!(text.contains("Cannot show image"));
    assert!(text.contains("Enter to open externally"));
    clear();
}

#[test]
fn empty_image_area_does_not_start_a_worker() {
    let source = image(1, 1, &[255, 0, 0, 255]);
    let mut terminal = Terminal::new(TestBackend::new(10, 2)).expect("terminal");
    draw_image(&mut terminal, Rect::new(0, 0, 0, 2), &source);
    RENDERER.with_borrow(|renderer| assert!(renderer.worker.is_none()));
}

#[test]
fn worker_start_failure_gives_an_actionable_message() {
    RENDERER.with_borrow_mut(|renderer| renderer.failed = true);
    let source = image(1, 1, &[255, 0, 0, 255]);
    let mut terminal = Terminal::new(TestBackend::new(72, 2)).expect("terminal");
    draw_image(&mut terminal, Rect::new(0, 0, 72, 2), &source);
    assert!(row(&terminal, 0).contains("Enter to open externally"));
    RENDERER.with_borrow_mut(|renderer| renderer.failed = false);
}

#[test]
fn resizing_requests_a_new_protocol_and_uses_the_new_area() {
    set_mode(Mode::Iterm2);
    let source = image(1, 1, &[255, 0, 0, 255]);
    let mut terminal = Terminal::new(TestBackend::new(30, 8)).expect("terminal");
    draw_image(&mut terminal, Rect::new(0, 0, 8, 4), &source);
    wait_for_protocol();
    draw_image(&mut terminal, Rect::new(0, 0, 8, 4), &source);
    let initial = terminal.backend().buffer()[(0, 0)].symbol();
    assert!(initial.contains("width=8;height=4;"));
    draw_image(&mut terminal, Rect::new(0, 0, 6, 3), &source);
    let preparing: String = "Preparing image".chars().take(6).collect();
    assert!(row(&terminal, 0).starts_with(&preparing));
    wait_for_protocol();
    draw_image(&mut terminal, Rect::new(0, 0, 6, 3), &source);
    let resized = terminal.backend().buffer()[(0, 0)].symbol();
    assert!(resized.contains("width=6;height=3;"));
    clear();
    set_mode(Mode::Halfblocks);
}

#[test]
fn repeated_draws_do_not_enqueue_the_same_pending_image_again() {
    set_mode(Mode::Halfblocks);
    let red = image(1, 1, &[255, 0, 0, 255]);
    let area = Rect::new(0, 0, 30, 4);
    let mut terminal = Terminal::new(TestBackend::new(30, 4)).expect("terminal");
    draw_image(&mut terminal, area, &red);
    draw_image(&mut terminal, area, &red);
    assert!(row(&terminal, 0).contains("Preparing image"));
    wait_for_protocol();
    draw_image(&mut terminal, area, &red);
    assert!(!pump(), "a repeated frame must not queue a second render");
    clear();
}
