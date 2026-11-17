//! Purpose: BDD steps for Far-style ` Reading: N files ` top-border progress and Esc partial listing.

use std::{cell::RefCell, fs, rc::Rc, time::Duration};

use cucumber::{given, then, when};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use super::BehaviorWorld;
use crate::{
    models::PanelSide,
    progress::{self, DirectoryRead, Painter, Snapshot},
    ui::reading_banner,
};

const TEAL_BG: Color = Color::Rgb(0, 132, 132);
const BLACK_FG: Color = Color::Rgb(0, 0, 0);

#[derive(Default)]
pub(super) struct Fixture {
    pub frames: Vec<DirectoryRead>,
}

#[derive(Clone, Default)]
struct BannerRecorder(Rc<RefCell<Vec<DirectoryRead>>>);

impl Painter for BannerRecorder {
    fn paint(&mut self, _snapshot: &Snapshot) {}

    fn paint_directory_read(&mut self, read: &DirectoryRead) {
        self.0.borrow_mut().push(*read);
    }
}

#[given(expr = "a subfolder {string} under the {word} panel with {int} files")]
fn create_populated_subfolder(
    world: &mut BehaviorWorld,
    folder: String,
    side: String,
    count: usize,
) {
    let case = world.case();
    let dir = case.root.path().join(&side).join(&folder);
    fs::create_dir_all(&dir).unwrap();
    for index in 0..count {
        fs::write(dir.join(format!("entry_{index:03}.txt")), b"content\n").unwrap();
    }
    case.app
        .panel_mut(super::Case::side(&side))
        .load_entries()
        .unwrap();
    case.paint();
}

#[when(expr = "I enter {string} on the left panel with immediate reading progress")]
fn enter_left_with_progress(world: &mut BehaviorWorld, folder: String) {
    record_navigation(world, PanelSide::Left, &folder, Duration::ZERO);
}

#[when(expr = "I switch to the right panel and enter {string} with immediate reading progress")]
fn enter_right_with_progress(world: &mut BehaviorWorld, folder: String) {
    world.case().press("Tab");
    record_navigation(world, PanelSide::Right, &folder, Duration::ZERO);
}

#[when(expr = "I enter {string} on the left panel within the 100 ms threshold")]
fn enter_left_fast(world: &mut BehaviorWorld, folder: String) {
    record_navigation(world, PanelSide::Left, &folder, Duration::from_secs(3600));
}

fn record_navigation(world: &mut BehaviorWorld, side: PanelSide, folder: &str, interval: Duration) {
    let recorder = BannerRecorder::default();
    let frames = Rc::clone(&recorder.0);
    {
        let _installed = progress::install_with(Box::new(recorder), interval);
        let case = world.case();
        let target = case.app.panel(side).path.join(folder);
        case.app.panel_mut(side).navigate_to(target).unwrap();
    }
    world.case().paint();
    world.directory_reading = Some(Fixture {
        frames: frames.borrow().clone(),
    });
}

#[then(expr = "the reading progress banner shows {string} on the {word} panel top border")]
fn verify_banner_on_panel_border(world: &mut BehaviorWorld, expected: String, side: String) {
    let frames = &world
        .directory_reading
        .as_ref()
        .expect("reading fixture recorded")
        .frames;
    let last = frames.last().expect("at least one reading banner painted");
    let expected_x = if side == "left" { 0 } else { 60 };
    assert_eq!(last.header.map(|header| header.x), Some(expected_x));
    assert_eq!(last.header.map(|header| header.y), Some(1));

    let banner = reading_banner(Rect::new(0, 0, 120, 30), last);
    let rendered = row_text(&banner);
    assert!(
        rendered.contains(&expected),
        "expected {expected:?} in {rendered:?}"
    );
    let badge_x =
        banner.area.x + u16::try_from(rendered.chars().position(|ch| ch == ' ').unwrap()).unwrap();
    let cell = &banner[(badge_x, banner.area.y)];
    assert_eq!(cell.fg, BLACK_FG);
    assert_eq!(cell.bg, TEAL_BG);
}

fn row_text(buffer: &Buffer) -> String {
    (buffer.area.left()..buffer.area.right())
        .map(|x| buffer[(x, buffer.area.y)].symbol())
        .collect()
}

#[then("no reading progress banner was painted")]
fn verify_no_banner_painted(world: &mut BehaviorWorld) {
    let frames = &world
        .directory_reading
        .as_ref()
        .expect("reading fixture recorded")
        .frames;
    assert_eq!(frames.len(), 0, "unexpected frames: {frames:?}");
}

#[when(expr = "I enter {string} on the left panel and press Esc after {int} files")]
fn enter_left_and_cancel(world: &mut BehaviorWorld, folder: String, limit: usize) {
    let case = world.case();
    let target = case.app.left_panel.path.join(&folder);
    let mut seen = 0usize;
    case.app
        .left_panel
        .navigate_to_with_cancel(target, |_| {
            seen += 1;
            seen >= limit
        })
        .unwrap();
    case.paint();
}

#[then(expr = "the left panel is inside {string} showing {string} and {int} files")]
fn verify_partial_left_panel(
    world: &mut BehaviorWorld,
    folder: String,
    parent_name: String,
    file_count: usize,
) {
    let panel = &world.case().app.left_panel;
    assert!(panel.path.ends_with(&folder), "path is {:?}", panel.path);
    assert_eq!(
        panel.entries.first().map(|entry| entry.name.as_str()),
        Some(parent_name.as_str())
    );
    assert_eq!(panel.entries.len(), file_count + 1);
}
