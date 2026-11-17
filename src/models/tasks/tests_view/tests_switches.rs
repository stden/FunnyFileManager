//! Purpose: unit tests for the view switches: kernel threads, the flat view and the
//! last column's text.

use super::*;

#[test]
fn kernel_threads_are_hidden_until_shown() {
    let kthreadd = ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(2, "kthreadd")
    };
    let snapshot = snapshot(vec![process(1, "init"), kthreadd]);
    let hidden = ViewOptions::default();
    assert_eq!(hidden.kernel_threads, KernelThreads::Hidden);
    assert_eq!(pids(&snapshot, &build_view(snapshot.rows(), &hidden)), [1]);

    let shown = ViewOptions {
        kernel_threads: KernelThreads::Hidden.toggled(),
        ..ViewOptions::default()
    };
    let mut both = pids(&snapshot, &build_view(snapshot.rows(), &shown));
    both.sort_unstable();
    assert_eq!(both, [1, 2]);
}

#[test]
fn a_flat_view_draws_no_guides_and_the_switches_toggle_back() {
    let snapshot = browsers();
    let view = build_view(snapshot.rows(), &ViewOptions::default());
    assert!(view.iter().all(|shown| shown.line() == TreeLine::Flat));
    assert_eq!(Arrangement::Flat.toggled(), Arrangement::Tree);
    assert_eq!(Arrangement::Tree.toggled(), Arrangement::Flat);
    assert_eq!(KernelThreads::Shown.toggled(), KernelThreads::Hidden);
    assert_eq!(CommandView::CommandLine.toggled(), CommandView::Name);
    assert_eq!(CommandView::Name.toggled(), CommandView::CommandLine);
}

#[test]
fn the_command_view_picks_the_last_columns_text() {
    let chromium = named(2, "chromium", "/usr/bin/chromium --incognito");
    assert_eq!(
        CommandView::CommandLine.text(&chromium),
        "/usr/bin/chromium --incognito"
    );
    assert_eq!(CommandView::Name.text(&chromium), "chromium");
}
