//! Purpose: Real cross-device moves finish a complete copy before removing sources.
#![cfg(target_os = "linux")]

use std::{fs, io, os::unix::fs::MetadataExt};

use funny_file_manager::handlers::handle_move;
use funny_file_manager::models::{App, FileName, Panel, PanelSide};

#[test]
fn a_move_between_filesystems_preserves_the_complete_tree() -> io::Result<()> {
    let from = tempfile::tempdir_in("/var/tmp")?;
    let Ok(to) = tempfile::tempdir_in("/dev/shm") else {
        return Ok(());
    };
    if fs::metadata(from.path())?.dev() == fs::metadata(to.path())?.dev() {
        return Ok(());
    }
    let source = from.path().join("tree");
    fs::create_dir_all(source.join("nested"))?;
    fs::write(source.join("nested/file"), b"complete")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(from.path().to_path_buf())?;
    app.right_panel = Panel::new(to.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    app.left_panel.marked.insert(FileName::from("tree"));

    handle_move(&mut app)?;

    assert!(
        !source.exists(),
        "only a complete copy may remove the source"
    );
    assert_eq!(fs::read(to.path().join("tree/nested/file"))?, b"complete");
    assert!(!app.overlay.is_active(), "{}", app.status);
    Ok(())
}

#[test]
fn a_cross_device_fifo_move_is_refused_without_creating_a_destination() -> io::Result<()> {
    use rustix::fs::{mkfifoat, Mode, CWD};

    let from = tempfile::tempdir_in("/var/tmp")?;
    let Ok(to) = tempfile::tempdir_in("/dev/shm") else {
        return Ok(());
    };
    if fs::metadata(from.path())?.dev() == fs::metadata(to.path())?.dev() {
        return Ok(());
    }
    let source = from.path().join("fifo");
    mkfifoat(CWD, &source, Mode::RUSR | Mode::WUSR)?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(from.path().to_path_buf())?;
    app.right_panel = Panel::new(to.path().to_path_buf())?;
    app.active_panel = PanelSide::Left;
    app.left_panel.marked.insert(FileName::from("fifo"));

    handle_move(&mut app)?;

    assert!(source.exists());
    assert_eq!(fs::read_dir(to.path())?.count(), 0);
    assert!(app.status.to_string().contains("1 error(s)"));
    Ok(())
}
