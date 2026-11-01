//! Resize, composite transparency, and encode away from the UI thread.

use std::fmt::Write as _;

use image::{imageops::FilterType, DynamicImage, Rgba, RgbaImage};
use ratatui::layout::Size;
use ratatui_image::protocol::{halfblocks::Halfblocks, iterm2::Iterm2, Protocol};

use super::{Key, Mode};

const CELL_WIDTH: u32 = 8;
const CELL_HEIGHT: u32 = 16;
const MAX_RENDER_SIDE: u32 = 2048;

pub(super) fn protocol(key: &Key) -> Option<Protocol> {
    let (image, size) = fitted(key)?;
    match key.mode {
        Mode::Halfblocks => Halfblocks::new(image, size).ok().map(Protocol::Halfblocks),
        Mode::Iterm2 => iterm2(image, size),
    }
}

fn iterm2(image: DynamicImage, size: Size) -> Option<Protocol> {
    let pixels = format!("width={}px;height={}px;", image.width(), image.height());
    // xterm.js otherwise derives height from width and fractional font metrics
    // can round into an extra row, scrolling when the image touches the bottom.
    let cells = format!(
        "width={};height={};preserveAspectRatio=0;",
        size.width, size.height
    );
    let mut protocol = Iterm2::new(image, size, false).ok()?;
    let (_, payload) = protocol.data.split_once("\x1b]1337;File=")?;
    // The upstream prefix moves down `height` times. At the bottom edge the
    // last move is clamped, so moving up by `height` shifts the image up one
    // row. Clear each row but move only between rows, then return to the top.
    let clear = format!("\x1b[{}X", size.width);
    let mut prefix = clear.clone();
    for _ in 1..size.height {
        prefix.push_str("\x1b[1B");
        prefix.push_str(&clear);
    }
    if size.height > 1 {
        write!(&mut prefix, "\x1b[{}A", size.height - 1).ok()?;
    }
    // xterm's iTerm protocol currently ignores doNotMoveCursor. Save/restore
    // the terminal cursor and style, then advance by the one cell Ratatui's
    // forced-width diff associates with this sequence.
    protocol.data =
        format!("{prefix}\x1b7\x1b]1337;File={payload}\x1b8\x1b[1C").replace(&pixels, &cells);
    Some(Protocol::ITerm2(protocol))
}

fn fitted(key: &Key) -> Option<(DynamicImage, Size)> {
    let image = &key.image;
    let rgba = RgbaImage::from_raw(image.width, image.height, image.pixels.to_vec())?;
    let mut source = DynamicImage::ImageRgba8(rgba);
    if key.zoom > 1 {
        let width = (image.width / u32::from(key.zoom)).max(1);
        let height = (image.height / u32::from(key.zoom)).max(1);
        let x = key.pan.0.min(image.width.saturating_sub(width));
        let y = key.pan.1.min(image.height.saturating_sub(height));
        source = source.crop_imm(x, y, width, height);
    }
    let width = (u32::from(key.size.width) * CELL_WIDTH).clamp(1, MAX_RENDER_SIDE);
    let height = (u32::from(key.size.height) * CELL_HEIGHT).clamp(1, MAX_RENDER_SIDE);
    let source = source
        .resize(width, height, FilterType::Triangle)
        .into_rgba8();
    let image = checkerboard(&source);
    let size = Size::new(
        u16::try_from(image.width().div_ceil(CELL_WIDTH)).ok()?,
        u16::try_from(image.height().div_ceil(CELL_HEIGHT)).ok()?,
    );
    Some((DynamicImage::ImageRgba8(image), size))
}

fn checkerboard(source: &RgbaImage) -> RgbaImage {
    let mut background = RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let shade = if (x / 8 + y / 8) % 2 == 0 { 192 } else { 128 };
        Rgba([shade, shade, shade, 255])
    });
    image::imageops::overlay(&mut background, source, 0, 0);
    background
}
