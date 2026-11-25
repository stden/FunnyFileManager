//! Purpose: Acceptance BDD steps for F8 parallel streaming delete, dialog rendering, and Esc cancel.

use std::{cell::RefCell, fs, path::Path, rc::Rc, time::Duration};

use cucumber::{given, then, when};
use ratatui::layout::Rect;

use super::BehaviorWorld;
use crate::{
    progress::{self, Operation, Painter, Phase, Snapshot, Tally},
    terminal::headless::HeadlessHost,
    test_support::make_dir_link,
    ui::progress_dialog,
};

#[derive(Default)]
pub(super) struct Fixture {
    pub frames: Vec<Snapshot>,
}

#[derive(Clone, Default)]
struct SnapshotCapture(Rc<RefCell<Vec<Snapshot>>>);

impl Painter for SnapshotCapture {
    fn paint(&mut self, snapshot: &Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    (buffer.area.y..buffer.area.y + buffer.area.height)
        .flat_map(|y| {
            (buffer.area.x..buffer.area.x + buffer.area.width).map(move |x| buffer[(x, y)].symbol())
        })
        .collect()
}

fn seed_nested_folder(dir: &Path, count: usize) {
    let deep = dir.join("sub").join("deep");
    fs::create_dir_all(&deep).unwrap();
    for index in 0..count {
        let target_dir = if index % 3 == 0 { &deep } else { dir };
        fs::write(target_dir.join(format!("item_{index:02}.dat")), b"cache").unwrap();
    }
}

#[given(
    expr = "a nested folder {string} under the left panel with {int} files and a directory link {string} keeping {string}"
)]
fn given_nested_folder_with_link(
    world: &mut BehaviorWorld,
    folder: String,
    count: usize,
    link_name: String,
    kept_name: String,
) {
    let left_root = world.case().root.path().join("left");
    let victim = left_root.join(&folder);
    seed_nested_folder(&victim, count);
    let kept = left_root.join(&kept_name);
    fs::create_dir_all(&kept).unwrap();
    fs::write(kept.join("readme.md"), b"do not delete").unwrap();
    make_dir_link(&kept, &victim.join(link_name)).unwrap();
    world.case().app.left_panel.load_entries().unwrap();
    world.case().paint();
}

#[given(expr = "a nested folder {string} under the left panel with {int} files")]
fn given_nested_folder(world: &mut BehaviorWorld, folder: String, count: usize) {
    let victim = world.case().root.path().join("left").join(folder);
    seed_nested_folder(&victim, count);
    world.case().app.left_panel.load_entries().unwrap();
    world.case().paint();
}

#[given(expr = "left panel entries {string} and {string} are marked")]
fn given_left_entries_marked(world: &mut BehaviorWorld, first: String, second: String) {
    let left_root = world.case().root.path().join("left");
    fs::write(left_root.join(&first), b"first").unwrap();
    fs::write(left_root.join(&second), b"second").unwrap();
    let panel = &mut world.case().app.left_panel;
    panel.load_entries().unwrap();
    panel.marked.insert(first.into());
    panel.marked.insert(second.into());
    world.case().paint();
}

fn dispatch_with_capture(world: &mut BehaviorWorld, keys: &str, interval: Duration) {
    let capture = SnapshotCapture::default();
    let frames = Rc::clone(&capture.0);
    {
        let _installed = progress::install_with(Box::new(capture), interval);
        world.case().press(keys);
    }
    world.delete_progress = Some(Fixture {
        frames: frames.borrow().clone(),
    });
}

#[when(expr = "I dispatch {string} with immediate delete progress capture")]
fn when_dispatch_immediate_delete(world: &mut BehaviorWorld, keys: String) {
    dispatch_with_capture(world, &keys, Duration::ZERO);
}

#[when(expr = "I dispatch {string} within the 100 ms delete threshold")]
fn when_dispatch_fast_delete(world: &mut BehaviorWorld, keys: String) {
    dispatch_with_capture(world, &keys, Duration::from_secs(3600));
}

#[when("I press F8 and interrupt deletion via headless cancel")]
fn when_f8_and_headless_cancel(world: &mut BehaviorWorld) {
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let _installed = progress::install_with(Box::new(SnapshotCapture::default()), Duration::ZERO);
    host.cancel();
    world.case().press("F8 Enter");
}

#[then(expr = "the kept target {string} still exists under the left panel")]
fn then_kept_target_exists(world: &mut BehaviorWorld, kept_name: String) {
    let path = world.case().root.path().join("left").join(kept_name);
    assert!(
        path.join("readme.md").is_file(),
        "target must survive link unlink"
    );
}

#[then(
    expr = "all {int} deleted items streamed in working phase and render the Delete dialog on normal and compact viewports"
)]
fn then_streamed_and_rendered(world: &mut BehaviorWorld, count: u64) {
    let frames = &world
        .delete_progress
        .as_ref()
        .expect("captured delete")
        .frames;
    assert!(frames.iter().all(|f| f.phase == Phase::Working));
    let last = frames.last().expect("at least one delete frame");
    let expected = Tally {
        files: count,
        bytes: 0,
    };
    assert_eq!(last.operation, Operation::Delete);
    assert_eq!((last.total, last.done), (expected, expected));
    for rect in [Rect::new(0, 0, 120, 30), Rect::new(0, 0, 80, 10)] {
        let rendered = buffer_text(&progress_dialog(rect, last));
        assert!(rendered.contains(" Delete "), "{rendered}");
        assert!(rendered.contains("Deleting the file"), "{rendered}");
    }
}

#[then(expr = "performance statistics record {string} and {string}")]
fn then_stats_record(world: &mut BehaviorWorld, first: String, second: String) {
    let report = super::statistics::saved(world.case()).to_string();
    assert!(report.contains(&first), "{report}");
    assert!(report.contains(&second), "{report}");
}

#[then("no delete progress frame was painted")]
fn then_no_delete_frame(world: &mut BehaviorWorld) {
    let frames = &world
        .delete_progress
        .as_ref()
        .expect("captured delete")
        .frames;
    assert_eq!(frames.len(), 0, "unexpected frames: {frames:?}");
}

#[then(expr = "the left file {string} still exists on disk and marks are cleared")]
fn then_left_file_kept_and_marks_cleared(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    assert!(case.root.path().join("left").join(name).is_file());
    assert!(case.app.left_panel.marked.is_empty());
}
