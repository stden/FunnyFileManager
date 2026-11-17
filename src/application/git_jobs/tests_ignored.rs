//! Purpose: Unit tests for background `git check-ignore` scheduling, deduplication, and cancellation.

use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::models::{
    DirEntry, DirectoryListing, EntryKind, IgnoredCandidate, IgnoredProbe, Panel, PanelMetadata,
};

const WAIT_LIMIT: Duration = Duration::from_secs(5);

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    }
}

fn panel_with_ignored_probe(path: &str, files: &[&str]) -> Panel {
    let mut panel = Panel::from_entries(PathBuf::from(path), Vec::new());
    let candidates = files
        .iter()
        .map(|name| IgnoredCandidate {
            name: FileName::from(*name),
            is_dir: false,
        })
        .collect();
    panel.apply_listing(DirectoryListing {
        entries: files
            .iter()
            .map(|name| entry(name, EntryKind::File))
            .collect(),
        parent: None,
        metadata: PanelMetadata {
            ignored_probe: IgnoredProbe::from_pending(candidates),
            ..PanelMetadata::default()
        },
    });
    panel
}

#[test]
fn deduplicates_shared_ignored_probe_across_both_panels_and_redraws_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_probe = Arc::clone(&calls);
    let mut jobs = GitDirtyJobs::with_ignored_prober(move |_dir, candidates| {
        calls_probe.fetch_add(1, Ordering::SeqCst);
        candidates
            .iter()
            .filter(|c| c.name.as_str() == "a.log")
            .map(|c| c.name.clone())
            .collect()
    });
    let mut app = App::new().expect("app");
    app.left_panel = panel_with_ignored_probe("/repo", &["a.log", "main.rs"]);
    app.right_panel = panel_with_ignored_probe("/repo", &["a.log", "main.rs"]);

    let deadline = Instant::now() + WAIT_LIMIT;
    let mut redraws = 0;
    while !app.left_panel.is_ignored("a.log") || !app.right_panel.is_ignored("a.log") {
        assert!(Instant::now() < deadline, "ignored probe timed out");
        if jobs.pump(&mut app) == Redraw::Needed {
            redraws += 1;
        }
        thread::yield_now();
    }
    assert_eq!(redraws, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!app.left_panel.is_ignored("main.rs"));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
}

#[test]
fn navigating_away_cancels_queued_ignored_probes_and_ignores_stale_results() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let seen = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
    let mut jobs = gated_ignored_worker(started_tx, release_rx, Arc::clone(&seen));

    let mut app = App::new().expect("app");
    app.left_panel = panel_with_ignored_probe("/first", &["a.log"]);
    app.right_panel = panel_with_ignored_probe("/queued", &["a.log"]);
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    started_rx.recv_timeout(WAIT_LIMIT).expect("worker started");

    app.left_panel = Panel::from_entries(
        PathBuf::from("/now-left"),
        vec![entry("a.log", EntryKind::File)],
    );
    app.right_panel = Panel::from_entries(
        PathBuf::from("/now-right"),
        vec![entry("a.log", EntryKind::File)],
    );
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);

    release_tx.send(()).expect("release");
    thread::sleep(Duration::from_millis(20));
    assert_eq!(jobs.pump(&mut app), Redraw::Unchanged);
    assert!(!app.left_panel.is_ignored("a.log"));
    assert_eq!(*seen.lock().expect("seen"), vec![PathBuf::from("/first")]);
}

fn gated_ignored_worker(
    started_tx: mpsc::Sender<()>,
    release_rx: mpsc::Receiver<()>,
    seen: Arc<Mutex<Vec<PathBuf>>>,
) -> GitDirtyJobs {
    let release_rx = Mutex::new(release_rx);
    GitDirtyJobs::with_ignored_workers(1, move |dir, _candidates| {
        seen.lock().expect("seen").push(dir.to_path_buf());
        if dir == Path::new("/first") {
            started_tx.send(()).expect("started");
            release_rx
                .lock()
                .expect("release")
                .recv_timeout(WAIT_LIMIT)
                .expect("wait");
        }
        BTreeSet::from([FileName::from("a.log")])
    })
}

#[test]
fn forced_ignored_worker_spawn_failure_degrades_gracefully() {
    let mut jobs = GitDirtyJobs::with_ignored_prober(|_dir, _candidates| {
        BTreeSet::from([FileName::from("a.log")])
    });
    let mut app = App::new().expect("app");
    app.left_panel = panel_with_ignored_probe("/repo", &["a.log"]);
    ignored::FORCE_IGNORED_SPAWN_FAILURE.set(true);
    let redraw = jobs.pump(&mut app);
    ignored::FORCE_IGNORED_SPAWN_FAILURE.set(false);
    assert_eq!(redraw, Redraw::Unchanged);
}

#[test]
fn poisoned_ignored_queue_reports_zero_demand() {
    let pool = ignored::IgnoredPool::new();
    let shared = Arc::clone(&pool.shared);
    let poisoner = thread::spawn(move || {
        let _held = shared.0.lock();
        panic!("poison ignored queue");
    });
    assert!(poisoner.join().is_err());

    let candidate = IgnoredCandidate {
        name: FileName::from("a.log"),
        is_dir: false,
    };
    let batch = Some((PathBuf::from("/repo"), vec![candidate]));
    assert_eq!(
        pool.update_queue([Path::new("/repo"), Path::new("/repo")], [batch, None]),
        0
    );
}
