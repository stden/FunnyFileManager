//! Bounded loading of PNG and JPEG images away from the event loop.

use std::{io::Cursor, path::Path, sync::Arc};

use image::{ImageDecoder, ImageFormat, ImageReader, Limits};

use crate::fileload::{read_regular, Loaded};
pub use crate::models::DecodedImage;

const MAX_COMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_DIMENSION: u32 = 16_384;
const MAX_DECODE_BYTES: u64 = 128 * 1024 * 1024;

/// Only known extensions enter image loading; content is verified by decoder.
pub fn is_supported_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("png")
                || ext.eq_ignore_ascii_case("jpg")
                || ext.eq_ignore_ascii_case("jpeg")
        })
}

/// Read only a regular bounded file, check dimensions before allocating pixels,
/// and honour image orientation metadata.
pub fn decode(path: &Path) -> Result<DecodedImage, String> {
    let bytes = image_bytes(path)?;
    decode_bytes(bytes)
}

fn image_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = match read_regular(path, MAX_COMPRESSED_BYTES).map_err(|e| e.to_string())? {
        Loaded::Bytes(bytes) => bytes,
        Loaded::NotRegular => return Err("Not a regular file".to_owned()),
        Loaded::TooLarge(_) => return Err("Image file exceeds 32 MiB".to_owned()),
    };
    Ok(bytes)
}

fn decode_bytes(bytes: Vec<u8>) -> Result<DecodedImage, String> {
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let format = match reader.format() {
        Some(ImageFormat::Png) => "PNG",
        Some(ImageFormat::Jpeg) => "JPEG",
        _ => return Err("Not a supported PNG or JPEG image".to_owned()),
    };
    reader.limits(decode_limits());
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_PIXELS || decoder.total_bytes() > MAX_DECODE_BYTES
    {
        return Err("Image dimensions exceed the viewing limit".to_owned());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    let image = image.into_rgba8();
    Ok(DecodedImage {
        width: image.width(),
        height: image.height(),
        format,
        size,
        pixels: Arc::from(image.into_raw()),
    })
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

#[cfg(test)]
mod tests;
