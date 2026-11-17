//! Image decode limits and format detection.

use super::*;
use image::ImageEncoder;

fn image_file(name: &str, format: ImageFormat) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join(name);
    let image = image::RgbaImage::from_pixel(2, 1, image::Rgba([220, 10, 20, 255]));
    image::DynamicImage::ImageRgba8(image)
        .save_with_format(&path, format)
        .expect("image fixture");
    (dir, path)
}

#[test]
fn supported_extensions_are_case_insensitive() {
    for name in ["photo.png", "photo.PNG", "photo.jpg", "photo.JPEG"] {
        assert!(is_supported_path(Path::new(name)), "{name}");
    }
    for name in ["photo.svg", "photo.png.bak", "png"] {
        assert!(!is_supported_path(Path::new(name)), "{name}");
    }
}

#[test]
fn png_and_jpeg_decode_with_metadata() {
    for (name, format, label) in [
        ("image.png", ImageFormat::Png, "PNG"),
        ("image.jpeg", ImageFormat::Jpeg, "JPEG"),
    ] {
        let (_dir, path) = image_file(name, format);
        let image = decode(&path).expect("image decodes");
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.format(), label);
        assert_eq!(
            image.size(),
            std::fs::metadata(path).expect("metadata").len()
        );
        assert_eq!(image.pixels.len(), 8);
    }
}

#[test]
fn invalid_and_oversized_files_are_refused() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("image.png");
    std::fs::write(&path, b"not an image").expect("fixture");
    assert!(decode(&path).is_err());
    std::fs::File::create(&path)
        .expect("file")
        .set_len(MAX_COMPRESSED_BYTES + 1)
        .expect("size");
    assert!(decode(&path).expect_err("large file").contains("32 MiB"));
    assert!(decode(dir.path())
        .expect_err("directory")
        .contains("regular"));
}

#[test]
fn oversized_dimensions_are_rejected_before_pixel_allocation() {
    use base64::Engine as _;
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("huge.png");
    let bytes = base64::engine::general_purpose::STANDARD.decode(
        "iVBORw0KGgoAAAANSUhEUgAATiAAAAABCAYAAAA7tJ6OAAAACUlEQVR4nGMAAAABAAFe/335AAAAAElFTkSuQmCC"
    ).expect("fixture");
    std::fs::write(&path, bytes).expect("fixture");
    assert!(decode(&path).is_err());
}

#[test]
fn excessive_pixel_count_is_rejected_even_with_valid_axis_lengths() {
    use base64::Engine as _;
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("too-many-pixels.png");
    // 5000 × 5000: each side fits MAX_DIMENSION, but the pixel count does not.
    let bytes = base64::engine::general_purpose::STANDARD.decode(
        "iVBORw0KGgoAAAANSUhEUgAAE4gAABOICAYAAABdmIfLAAAACUlEQVR4nGMAAAABAAFe/335AAAAAElFTkSuQmCC"
    ).expect("fixture");
    std::fs::write(&path, bytes).expect("fixture");
    assert!(decode(&path)
        .expect_err("too many pixels")
        .contains("dimensions"));
}

#[test]
fn damaged_image_and_missing_file_are_reported() {
    let (dir, valid) = image_file("valid.png", ImageFormat::Png);
    let data = std::fs::read(valid).expect("fixture");
    let broken = dir.path().join("broken.png");
    std::fs::write(&broken, &data[..20]).expect("truncated image");
    assert!(decode(&broken).is_err());
    assert!(decode(&dir.path().join("missing.png")).is_err());
}

#[test]
fn image_format_comes_from_content_even_when_extension_differs() {
    let (dir, jpeg) = image_file("valid.jpeg", ImageFormat::Jpeg);
    let renamed = dir.path().join("renamed.png");
    std::fs::copy(jpeg, &renamed).expect("fixture");
    assert_eq!(decode(&renamed).expect("JPEG content").format(), "JPEG");
}

#[cfg(unix)]
#[test]
fn fifo_is_refused_without_opening_and_symlinks_follow_regular_files() {
    use std::os::unix::fs::symlink;
    let (dir, target) = image_file("source.png", ImageFormat::Png);
    let linked = dir.path().join("linked.png");
    symlink(&target, &linked).expect("symlink");
    assert_eq!(decode(&linked).expect("linked image").width(), 2);
    let fifo = dir.path().join("pipe.png");
    assert!(std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo")
        .success());
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(send.send(decode(&fifo)));
    });
    let result = receive
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("FIFO must not block");
    assert!(result.expect_err("FIFO").contains("regular"));
}

#[test]
fn jpeg_exif_orientation_is_applied() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("rotated.jpg");
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .write_image(
            &[255, 0, 0, 0, 255, 0],
            2,
            1,
            image::ExtendedColorType::Rgb8,
        )
        .expect("JPEG fixture");
    // Exif TIFF with orientation = 6 (rotate 90 degrees clockwise).
    let exif = [
        0xff, 0xe1, 0, 34, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0, 42, 0, 0, 0, 8, 0, 1, 1,
        0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0, 0, 0, 0,
    ];
    jpeg.splice(2..2, exif);
    std::fs::write(&path, jpeg).expect("fixture");
    let image = decode(&path).expect("oriented image");
    assert_eq!((image.width(), image.height()), (1, 2));
}
