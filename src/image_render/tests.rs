//! Protocol generation and bounded background rendering.

use super::*;
use ratatui::{backend::TestBackend, layout::Rect, style::Color, Terminal};
use ratatui_image::Image;
use std::time::{Duration, Instant};

mod tests_zoom;

fn key(mode: Mode, pixels: &[u8]) -> Key {
    Key {
        image: DecodedImage {
            width: 1,
            height: 1,
            format: "PNG",
            size: 10,
            pixels: Arc::from(pixels),
        },
        size: Size::new(8, 4),
        zoom: 1,
        pan: (0, 0),
        mode,
    }
}

#[test]
fn protocol_choice_is_explicit_and_unknown_settings_fall_back() {
    assert_eq!(Mode::from_setting("iterm2"), Mode::Iterm2);
    assert_eq!(Mode::from_setting("ITERM2"), Mode::Iterm2);
    assert_eq!(Mode::from_setting(""), Mode::Halfblocks);
    assert_eq!(Mode::from_setting("unknown"), Mode::Halfblocks);
}

#[test]
fn iterm2_protocol_encodes_inline_graphics() {
    let protocol = build::protocol(&key(Mode::Iterm2, &[255, 0, 0, 255])).expect("protocol");
    let Protocol::ITerm2(image) = protocol else {
        panic!("expected iTerm2 protocol");
    };
    assert!(image.data.contains("\x1b]1337;File=inline=1;"));
    assert!(image.data.ends_with("\x07\x1b8\x1b[1C"));
    assert!(image.data.contains(";preserveAspectRatio=0;"));
    assert!(
        image.data.contains(&format!(
            "width={};height={};",
            image.size.width, image.size.height
        )),
        "terminal cells, not assumed font pixel dimensions, control the display area"
    );
}

#[test]
fn iterm2_clears_its_area_without_moving_past_the_bottom_row() {
    let protocol = build::protocol(&key(Mode::Iterm2, &[255, 0, 0, 255])).expect("protocol");
    let Protocol::ITerm2(image) = protocol else {
        panic!("expected iTerm2 protocol");
    };
    assert_eq!(image.size.height, 4);
    let before_image = image.data.split("\x1b]1337;File=").next().expect("prefix");
    assert_eq!(before_image.matches("\x1b[1B").count(), 3);
    assert!(before_image.ends_with("\x1b[3A\x1b7"));
}

#[test]
fn one_row_iterm2_image_does_not_move_the_cursor() {
    let mut request = key(Mode::Iterm2, &[255, 0, 0, 255]);
    request.size = Size::new(8, 1);
    let protocol = build::protocol(&request).expect("protocol");
    let Protocol::ITerm2(image) = protocol else {
        panic!("expected iTerm2 protocol");
    };
    assert_eq!(image.size.height, 1);
    let before_image = image.data.split("\x1b]1337;File=").next().expect("prefix");
    assert!(!before_image.contains("\x1b[1B"));
    assert!(!before_image.contains("\x1b[0A"));
}

#[test]
fn halfblocks_render_and_composite_transparency() {
    let protocol = build::protocol(&key(Mode::Halfblocks, &[255, 0, 0, 0])).expect("protocol");
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).expect("terminal");
    terminal
        .draw(|frame| frame.render_widget(Image::new(&protocol), Rect::new(0, 0, 8, 4)))
        .expect("draw");
    let cell = &terminal.backend().buffer()[(0, 0)];
    assert_ne!(
        cell.bg,
        Color::Rgb(255, 0, 0),
        "transparent red is not painted opaque"
    );
    assert_ne!(
        cell.fg,
        Color::Rgb(255, 0, 0),
        "transparent red is not painted opaque"
    );
}

#[test]
fn worker_publishes_the_latest_requested_frame() {
    let mut worker = Worker::start().expect("worker");
    let old = key(Mode::Halfblocks, &[255, 0, 0, 255]);
    let latest = key(Mode::Halfblocks, &[0, 255, 0, 255]);
    worker.request(old);
    worker.request(latest.clone());
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        worker.pump();
        if worker
            .cache
            .iter()
            .any(|(key, value)| *key == latest && value.is_some())
        {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("latest frame was not delivered");
}

#[test]
fn clearing_releases_cached_pixels() {
    let mut worker = Worker::start().expect("worker");
    let job = key(Mode::Halfblocks, &[5, 6, 7, 255]);
    let pixels = Arc::downgrade(&job.image.pixels);
    worker.request(job);
    let deadline = Instant::now() + Duration::from_secs(5);
    while worker.cache.is_empty() && Instant::now() < deadline {
        worker.pump();
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!worker.cache.is_empty(), "fixture is cached");
    worker.clear();
    assert!(worker.cache.is_empty());
    assert!(pixels.upgrade().is_none(), "closed images release pixels");
}

#[test]
fn clearing_discards_old_work_without_blocking_the_next_image() {
    let mut worker = Worker::start().expect("worker");
    let old = key(Mode::Halfblocks, &[255, 0, 0, 255]);
    worker.request(old.clone());
    worker.clear();
    let current = key(Mode::Iterm2, &[0, 255, 0, 255]);
    worker.request(current.clone());
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        worker.pump();
        assert!(!worker.cache.iter().any(|(key, _)| *key == old));
        if worker.cache.iter().any(|(key, _)| *key == current) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("new image was not delivered");
}
