//! Purpose: Panel navigation and reload support Esc interruption and panel header tracking.

use std::{fs, io, time::Duration};

use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

use crate::{
    models::{App, Panel, PanelView},
    progress::{self, DirectoryRead, Painter, PanelHeader, Snapshot},
};

#[derive(Default)]
struct SilentPainter;

impl Painter for SilentPainter {
    fn paint(&mut self, _snapshot: &Snapshot) {}
    fn paint_directory_read(&mut self, _read: &DirectoryRead) {}
}

#[test]
fn navigate_and_reload_with_cancel_keep_partial_listing_and_parent() -> io::Result<()> {
    let temp = TempDir::new()?;
    let huge = temp.path().join("huge");
    fs::create_dir(&huge)?;
    for index in 0..6 {
        fs::write(huge.join(format!("file_{index}.txt")), b"payload")?;
    }
    fs::write(huge.join("descript.ion"), "file_0.txt Detailed note\n")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.view = PanelView::Descriptions;

    let mut count = 0usize;
    panel.navigate_to_with_cancel(huge.clone(), |_| {
        count += 1;
        count >= 3
    })?;
    assert_eq!(panel.path, huge);
    assert_eq!(panel.entries.first().map(|e| e.name.as_str()), Some(".."));
    assert_eq!(panel.entries.len(), 4);
    assert_eq!(panel.details.descriptions.len(), 0);

    let mut reload_count = 0usize;
    panel.load_entries_with_cancel(|_| {
        reload_count += 1;
        reload_count >= 2
    })?;
    assert_eq!(panel.entries.len(), 3);
    assert!(panel
        .navigate_to_with_cancel(temp.path().join("missing"), |_| false)
        .is_err());
    Ok(())
}

#[test]
fn live_cancel_throttles_polls_and_stops_when_headless_host_cancels() -> io::Result<()> {
    let temp = TempDir::new()?;
    let target = temp.path().join("slow");
    fs::create_dir(&target)?;
    for index in 0..4 {
        fs::write(target.join(format!("part_{index}.txt")), b"data")?;
    }

    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let _installed = progress::install_with(Box::new(SilentPainter), Duration::ZERO);

    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.navigate_to(target)?;
    assert_eq!(panel.entries.len(), 5, "uncurtailed read keeps all 4 files");

    host.cancel();
    panel.load_entries()?;
    assert_eq!(
        panel.entries.len(),
        2,
        "cancelled read stops after the first file plus .."
    );
    Ok(())
}

#[test]
fn swapping_panels_keeps_each_screen_sides_frame_header() -> io::Result<()> {
    let temp = TempDir::new()?;
    let left_dir = temp.path().join("left");
    let right_dir = temp.path().join("right");
    fs::create_dir(&left_dir)?;
    fs::create_dir(&right_dir)?;

    let mut app = App::with_panels(Panel::new(left_dir)?, Panel::new(right_dir)?);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("draw");

    let left_expected = Some(PanelHeader {
        x: 0,
        y: 1,
        width: 60,
    });
    let right_expected = Some(PanelHeader {
        x: 60,
        y: 1,
        width: 60,
    });
    assert_eq!(app.left_panel.frame_header(), left_expected);
    assert_eq!(app.right_panel.frame_header(), right_expected);

    app.swap_panels();
    assert_eq!(app.left_panel.frame_header(), left_expected);
    assert_eq!(app.right_panel.frame_header(), right_expected);
    Ok(())
}

#[test]
fn frame_header_accessors_store_and_swap_coordinates() {
    let left = Panel::from_entries("left".into(), Vec::new());
    let right = Panel::from_entries("right".into(), Vec::new());
    let first = PanelHeader {
        x: 0,
        y: 1,
        width: 50,
    };
    let second = PanelHeader {
        x: 50,
        y: 1,
        width: 50,
    };
    left.set_frame_header(first);
    right.set_frame_header(second);
    left.swap_frame_header(&right);
    assert_eq!(left.frame_header(), Some(second));
    assert_eq!(right.frame_header(), Some(first));
}
