//! Purpose: Unit tests for image editor launch, association fallback, and headless routing.

use super::*;
use crate::models::Panel;

fn ok_handler(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn no_association(_path: &Path) -> io::Result<()> {
    Err(io::Error::from_raw_os_error(ERROR_NO_ASSOCIATION))
}

fn missing_file(_path: &Path) -> io::Result<()> {
    Err(io::Error::from_raw_os_error(2))
}

#[test]
fn fallback_opens_only_when_edit_verb_has_no_association() {
    let path = Path::new("photo.jpg");
    assert!(edit_with_fallback(path, ok_handler, missing_file).is_ok());
    assert!(edit_with_fallback(path, no_association, ok_handler).is_ok());
    assert_eq!(
        edit_with_fallback(path, missing_file, ok_handler)
            .unwrap_err()
            .raw_os_error(),
        Some(2)
    );
}

#[test]
fn edit_path_routes_interactive_forced_error_and_headless() {
    let temp = tempfile::TempDir::new().unwrap();
    let image = temp.path().join("denis.jpg");
    std::fs::write(&image, b"jpg").unwrap();
    let panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(panel.clone(), panel);

    drop(take_edited());
    edit_path(&app, &image).unwrap();
    assert_eq!(take_edited(), Some(std::path::absolute(&image).unwrap()));

    fail_next_edit();
    assert!(edit_path(&app, &image).is_err());

    app.live.terminal_mode = TerminalMode::Headless;
    assert_eq!(
        edit_path(&app, &image).unwrap_err().kind(),
        io::ErrorKind::NotConnected
    );
}

#[cfg(not(windows))]
#[test]
fn unix_command_prefers_image_editor_and_falls_back_to_system_opener() {
    let temp = tempfile::TempDir::new().unwrap();
    let script = temp.path().join("my image editor.sh");
    std::fs::write(&script, "#!/bin/sh\n").unwrap();
    let target = temp.path().join("photo.png");

    let empty = crate::test_support::EnvVarGuard::set("IMAGE_EDITOR", Path::new(""));
    let fallback = unix_command(&target);
    drop(empty);
    assert_ne!(fallback.get_program(), script.as_os_str());

    let args =
        crate::test_support::EnvVarGuard::set("IMAGE_EDITOR", Path::new("gimp --new-instance"));
    let custom = unix_command(&target);
    drop(args);
    assert_eq!(custom.get_program(), "gimp");
    assert_eq!(
        custom
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec!["--new-instance".to_owned(), target.display().to_string()]
    );

    let spaced = crate::test_support::EnvVarGuard::set("IMAGE_EDITOR", &script);
    let file_cmd = unix_command(&target);
    drop(spaced);
    assert_eq!(file_cmd.get_program(), script.as_os_str());
}
