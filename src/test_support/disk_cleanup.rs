//! Purpose: Isolated cleanup fixtures shared by operation, popup, and rendering tests.

use std::path::Path;

use crate::models::{
    App, CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog, Overlay, Panel,
};

pub fn candidate(label: &'static str, path: &Path) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path: path.to_path_buf(),
        kind: CleanupKind::Path,
        size: None,
        checked: true,
        exists: true,
    }
}

pub fn heap_dumps(path: &Path) -> CleanupCandidate {
    CleanupCandidate {
        kind: CleanupKind::HeapDumps,
        ..candidate("JVM heap dumps (*.hprof)", path)
    }
}

pub fn dialog(candidates: Vec<CleanupCandidate>) -> DiskCleanupDialog {
    DiskCleanupDialog {
        candidates,
        field: CleanupField::Item(0),
    }
}

pub fn app(root: &Path, candidates: Vec<CleanupCandidate>) -> App {
    let panel = Panel::from_entries(root.to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::DiskCleanup(Box::new(dialog(candidates))));
    app
}

pub fn run(app: &mut App) {
    let _guard = crate::utils::refresh_panels_mutex()
        .lock()
        .expect("refresh lock");
    crate::ops::disk_cleanup::run(app);
}
