use super::*;
use std::io::Write;

#[test]
fn test_writable_test_backend_write() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    let data = b"test data";
    let written = backend.write(data).expect("test setup");
    assert_eq!(written, data.len());
    Ok(())
}

#[test]
fn test_writable_test_backend_flush() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    Write::flush(&mut backend).expect("test setup");
    Backend::flush(&mut backend).expect("test setup");
    Ok(())
}

#[test]
fn test_writable_test_backend_cursor_operations() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);

    let pos = backend.get_cursor_position().expect("test setup");
    assert_eq!((pos.x, pos.y), (0, 0));

    backend
        .set_cursor_position(Position::new(10, 5))
        .expect("test setup");
    let pos = backend.get_cursor_position().expect("test setup");
    assert_eq!((pos.x, pos.y), (10, 5));

    Ok(())
}

#[test]
fn test_writable_test_backend_clear() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    backend.clear().expect("test setup");
    Ok(())
}

#[test]
fn test_writable_test_backend_window_size() -> io::Result<()> {
    let mut backend = WritableTestBackend::new(80, 24);
    let window_size = backend.window_size().expect("test setup");

    assert_eq!(window_size.columns_rows.width, 80);
    assert_eq!(window_size.columns_rows.height, 24);
    assert_eq!(window_size.pixels.width, 80 * 8);
    assert_eq!(window_size.pixels.height, 24 * 16);

    Ok(())
}
