//! F3 opens supported images in its built-in viewer.

use super::*;
use base64::Engine as _;

#[test]
fn f3_opens_png_instead_of_reporting_it_as_binary() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("red.png");
    let png = base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==")
        .expect("PNG fixture");
    fs::write(&path, png)?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);

    view_path(&mut app, path.clone())?;

    assert!(app.get_viewer().is_some(), "F3 should open a PNG");
    assert_eq!(app.status, Status::Viewing(path.display().to_string()));
    Ok(())
}

#[test]
fn f3_measures_a_directory_named_like_a_png() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("folder.png");
    fs::create_dir(&path)?;
    fs::write(path.join("inside"), b"abc")?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("folder.png"));
    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());
    let entry = app.left_panel.get_selected_entry().expect("directory");
    assert_eq!(entry.size.bytes(), 3);
    Ok(())
}

#[test]
fn f3_opens_a_large_selected_image_without_reading_it_in_the_event_handler() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("large.png");
    fs::File::create(&path)?.set_len(512 * 1024 * 1024)?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("large.png"));
    handle_view(&mut app)?;
    assert!(matches!(
        app.get_viewer().and_then(|viewer| viewer.image.as_ref()),
        Some(crate::models::ImageState::Pending)
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn f3_measures_a_symlink_to_a_directory_named_like_a_png() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("folder");
    fs::create_dir(&path)?;
    fs::write(path.join("inside"), b"abc")?;
    std::os::unix::fs::symlink(&path, temp.path().join("link.png"))?;
    let panel = crate::models::Panel::new(temp.path().to_path_buf())?;
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(app.left_panel.select_named("link.png"));
    handle_view(&mut app)?;
    assert!(app.get_viewer().is_none());
    Ok(())
}
