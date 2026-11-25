//! Manual production-path benchmark for applying a directory's Git results.

use std::{
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::{DirEntry, FileName, Panel};
use crate::models::EntryKind;

#[test]
#[ignore = "manual release benchmark; reports timings without a machine-specific threshold"]
fn dirty_probe_batch_scaling() {
    for count in [1_000, 5_000, 10_000] {
        let names: Vec<FileName> = (0..count)
            .map(|index| FileName::new(format!("repository-{index:05}")))
            .collect();
        for round in 0..3 {
            let mut panel = fixture(&names);
            let dirty_elapsed = measure(&mut panel, &names, true);
            let clean_elapsed = measure(&mut panel, &names, false);
            eprintln!(
                "dirty_probe count={count} round={round}: dirty={dirty_elapsed:?} clean={clean_elapsed:?}"
            );
        }
    }
}

fn fixture(names: &[FileName]) -> Panel {
    let entries = names
        .iter()
        .map(|name| DirEntry {
            name: name.as_str().to_owned(),
            kind: EntryKind::Directory,
            ..DirEntry::default()
        })
        .collect();
    Panel::from_entries(PathBuf::from("/workspace/repositories"), entries)
}

fn measure(panel: &mut Panel, names: &[FileName], dirty: bool) -> Duration {
    let parent = panel.path.clone();
    let started = Instant::now();
    let changes = names
        .iter()
        .filter(|name| panel.apply_dirty_probe(black_box(&parent), black_box(name), dirty))
        .count();
    let elapsed = started.elapsed();
    assert_eq!(changes, names.len());
    assert!(names
        .iter()
        .all(|name| panel.is_dirty_repo(name.as_str()) == dirty));
    elapsed
}
