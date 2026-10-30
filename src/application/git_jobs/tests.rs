//! Purpose: Unit tests for background dirty-repository scheduling, deduplication, and cancellation.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::models::{DirEntry, DirectoryListing, DirtyProbe, EntryKind, Panel, PanelMetadata};

const WAIT_LIMIT: Duration = Duration::from_secs(5);

fn dir_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    }
}

fn panel_with_pending(path: &str, repos: &[&str]) -> Panel {
    let mut panel = Panel::from_entries(PathBuf::from(path), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: repos.iter().map(|name| dir_entry(name)).collect(),
        parent: None,
        metadata: PanelMetadata {
            dirty_probe: DirtyProbe::from_pending(
                repos.iter().map(|name| FileName::from(*name)).collect(),
            ),
            ..PanelMetadata::default()
        },
    });
    panel
}

#[test]
fn deduplicates_shared_directory_across_both_panels_and_redraws_only_on_change() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_probe = Arc::clone(&calls);
    let mut jobs = GitDirtyJobs::with_prober(move |_parent, name| {
        calls_probe.fetch_add(1, Ordering::SeqCst);
        name.as_str() == "dirty-repo"
    });
    let mut app = App::new().expect("app");
    app.left_panel = panel_with_pending("/work", &["clean-repo", "dirty-repo"]);
    app.right_panel = panel_with_pending("/work", &["clean-repo", "dirty-repo"]);

    let redraws = wait_for_dirty(&mut jobs, &mut app, &calls);
    assert_eq!(redraws, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(!app.left_panel.is_dirty_repo("clean-repo"));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
}

fn wait_for_dirty(jobs: &mut GitDirtyJobs, app: &mut App, calls: &AtomicUsize) -> usize {
    let deadline = Instant::now() + WAIT_LIMIT;
    let mut redraws = 0;
    while !app.left_panel.is_dirty_repo("dirty-repo")
        || !app.right_panel.is_dirty_repo("dirty-repo")
        || calls.load(Ordering::SeqCst) < 2
    {
        assert!(
            Instant::now() < deadline,
            "background probes did not finish"
        );
        if jobs.pump(app) == Redraw::Needed {
            redraws += 1;
        }
        thread::yield_now();
    }
    redraws
}

#[test]
fn navigating_away_cancels_queued_probes_and_ignores_in_flight_stale_results() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut jobs = gated_single_worker(started_tx, release_rx, Arc::clone(&seen));

    let mut app = App::new().expect("app");
    app.left_panel = panel_with_pending("/old", &["first"]);
    app.right_panel = Panel::from_entries(PathBuf::from("/elsewhere"), Vec::new());

    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    started_rx.recv_timeout(WAIT_LIMIT).expect("worker started");

    app.left_panel = panel_with_pending("/old", &["first", "queued-a", "queued-b"]);
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    app.left_panel = Panel::from_entries(PathBuf::from("/new"), vec![dir_entry("first")]);
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);

    release_tx.send(()).expect("release first");
    thread::sleep(Duration::from_millis(20));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    assert!(!app.left_panel.is_dirty_repo("first"));
    assert_eq!(*seen.lock().expect("seen"), vec!["first".to_owned()]);
}

fn gated_single_worker(
    started_tx: mpsc::Sender<()>,
    release_rx: mpsc::Receiver<()>,
    seen: Arc<Mutex<Vec<String>>>,
) -> GitDirtyJobs {
    let release_rx = Mutex::new(release_rx);
    GitDirtyJobs::with_workers(1, move |_parent, name| {
        seen.lock().expect("seen").push(name.as_str().to_owned());
        if name.as_str() == "first" {
            started_tx.send(()).expect("signal started");
            release_rx
                .lock()
                .expect("release lock")
                .recv_timeout(WAIT_LIMIT)
                .expect("wait release");
        }
        true
    })
}
