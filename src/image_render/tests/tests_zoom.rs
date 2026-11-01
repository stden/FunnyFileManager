//! Cropping a zoomed image follows pan and clamps at image edges.

use super::*;

fn image() -> Key {
    let mut image = key(Mode::Halfblocks, &[0, 0, 0, 255]);
    image.image.width = 4;
    image.image.height = 2;
    image.image.pixels = Arc::from([
        255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 0, 0,
        255, 0, 255, 0, 255, 0, 255, 0, 255,
    ]);
    image.zoom = 2;
    image
}

fn background(key: &Key) -> Color {
    let protocol = build::protocol(key).expect("protocol");
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).expect("terminal");
    terminal
        .draw(|frame| frame.render_widget(Image::new(&protocol), Rect::new(0, 0, 8, 4)))
        .expect("draw");
    terminal.backend().buffer()[(0, 0)].bg
}

#[test]
fn pan_changes_visible_half_and_clamps_out_of_bounds() {
    let mut request = image();
    assert_eq!(background(&request), Color::Rgb(255, 0, 0));
    request.pan = (u32::MAX, u32::MAX);
    assert_eq!(background(&request), Color::Rgb(0, 255, 0));
}
