//! Purpose: Unit tests for parallel directory deletion, streaming progress,
//! read-only and link safety, and Esc / Ctrl+C cancellation.

use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;

use super::tests_progress::{app_on, Recorder};
use crate::models::{App, Overlay, Panel};
use crate::progress::{install_with, Operation, Painter, Phase, Snapshot, Tally};
use crate::status::Status;

#[derive(Default)]
struct SilentPainter;

impl Painter for SilentPainter {
    fn paint(&mut self, _snapshot: &Snapshot) {}
}

fn populate_deep_tree(root: &Path) -> u64 {
    let mut files = 0_u64;
    for branch in 0..18_u32 {
        let dir = root.join(format!("branch_{branch:02}"));
        let leaf = dir.join("leaf");
        fs::create_dir_all(&leaf).expect("mkdir");
        for idx in 0..20_u32 {
            fs::write(dir.join(format!("item_{idx:02}.txt")), b"payload").expect("write");
            files += 1;
        }
        for idx in 0..2_u32 {
            fs::write(leaf.join(format!("deep_{idx:02}.txt")), b"leaf").expect("write");
            files += 1;
        }
    }
    files
}

#[cfg(windows)]
fn add_readonly_branch(victim: &Path) -> u64 {
    let ro_dir = victim.join("ro_sub");
    fs::create_dir(&ro_dir).expect("mkdir ro_sub");
    let ro_file = ro_dir.join("ro_file.txt");
    fs::write(&ro_file, b"ro").expect("write ro_file");
    let mut perms = fs::metadata(&ro_file).expect("meta").permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro_file, perms.clone()).expect("set ro file");
    fs::set_permissions(&ro_dir, perms).expect("set ro dir");
    1
}

#[cfg(not(windows))]
const fn add_readonly_branch(_victim: &Path) -> u64 {
    0
}

fn marked_app(dir: &Path, names: &[&str]) -> io::Result<App> {
    let mut panel = Panel::new(dir.to_path_buf())?;
    for &name in names {
        panel.marked.insert(name.into());
    }
    Ok(App::with_panels(panel, Panel::new(dir.to_path_buf())?))
}

#[test]
fn parallel_delete_streams_progress_without_scanning_and_preserves_link_targets() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("tempdir");
    let kept = temp.path().join("kept_target");
    fs::create_dir_all(&kept).expect("mkdir kept");
    fs::write(kept.join("precious.txt"), b"keep me").expect("write kept");

    let victim = temp.path().join("victim");
    let expected = populate_deep_tree(&victim) + 1 + add_readonly_branch(&victim);
    crate::test_support::make_dir_link(&kept, &victim.join("dir_link")).expect("dir link");

    let mut app = app_on(temp.path(), "victim", temp.path());
    crate::ops::handle_delete_confirm(&mut app).expect("confirm");
    crate::ops::handle_delete(&mut app).expect("delete");

    assert!(!victim.exists() && kept.join("precious.txt").exists());
    assert_eq!(app.status, Status::Deleted { count: 1 });
    assert!(matches!(app.overlay, Overlay::None));
    let frames = recorder.frames();
    assert!(frames.iter().all(|frame| frame.phase == Phase::Working));
    let last = frames.last().expect("painted frames");
    let expected_tally = Tally {
        files: expected,
        bytes: 0,
    };
    assert_eq!(last.operation, Operation::Delete);
    assert_eq!((last.total, last.done), (expected_tally, expected_tally));
}

#[test]
fn handle_delete_with_cancel_stops_tree_cleanly_without_failure_modal() -> io::Result<()> {
    let (_recorder, _installed) = Recorder::install();
    let temp = TempDir::new()?;
    let victim = temp.path().join("tree");
    populate_deep_tree(&victim);

    let mut app = app_on(temp.path(), "tree", temp.path());
    crate::ops::handle_delete_confirm(&mut app)?;
    let mut checks = 0_usize;
    crate::ops::handle_delete_with_cancel(&mut app, |painted| {
        checks += usize::from(painted);
        checks >= 1
    })?;

    assert_eq!(app.status, Status::DeleteCancelled);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(victim.exists());
    Ok(())
}

#[test]
fn handle_delete_with_cancel_stops_between_marked_files() -> io::Result<()> {
    let (_recorder, _installed) = Recorder::install();
    let temp = TempDir::new()?;
    let (file_a, file_b) = (temp.path().join("aa.txt"), temp.path().join("zz.txt"));
    fs::write(&file_a, b"1")?;
    fs::write(&file_b, b"2")?;
    let mut app = marked_app(temp.path(), &["aa.txt", "zz.txt"])?;
    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete_with_cancel(&mut app, |painted| painted)?;

    assert!(!file_a.exists() && file_b.exists());
    assert_eq!(app.status, Status::DeleteCancelled);
    assert!(app.left_panel.marked.is_empty());
    Ok(())
}

#[test]
fn live_cancel_ignores_unshown_delete_and_throttles_shown_polls() -> io::Result<()> {
    let temp = TempDir::new()?;
    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();

    let fast = temp.path().join("fast.txt");
    fs::write(&fast, b"x")?;
    host.cancel();
    let mut app = app_on(temp.path(), "fast.txt", temp.path());
    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete(&mut app)?;
    assert!(!fast.exists());
    host.reset()?;

    let _installed = install_with(Box::new(SilentPainter), Duration::ZERO);
    fs::write(temp.path().join("one.txt"), b"1")?;
    fs::write(temp.path().join("two.txt"), b"2")?;
    let mut app = marked_app(temp.path(), &["one.txt", "two.txt"])?;
    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete(&mut app)?;
    assert_eq!(app.status, Status::Deleted { count: 2 });
    Ok(())
}

#[test]
fn live_cancel_stops_shown_delete_when_headless_host_cancels() -> io::Result<()> {
    let temp = TempDir::new()?;
    let host = crate::terminal::headless::HeadlessHost::new();
    let _scope = host.enter();
    let _installed = install_with(Box::new(SilentPainter), Duration::ZERO);

    let (first, second) = (temp.path().join("a.txt"), temp.path().join("b.txt"));
    fs::write(&first, b"a")?;
    fs::write(&second, b"b")?;
    let mut app = marked_app(temp.path(), &["a.txt", "b.txt"])?;
    host.cancel();
    crate::ops::handle_delete_confirm(&mut app)?;
    crate::ops::handle_delete(&mut app)?;

    assert!(!first.exists() && second.exists());
    assert_eq!(app.status, Status::DeleteCancelled);
    assert!(matches!(app.overlay, Overlay::None));
    Ok(())
}
