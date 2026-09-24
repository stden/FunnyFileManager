//! Unit tests: Alt+Del wipes the way Far describes — zeros over every byte,
//! then a truncation, a temporary name, and only then removal; directories
//! recurse and links are unlinked, never followed.
//!
//! The zeros are the point and the hardest thing to see: the next step erases
//! them. The `WITNESS` seam is the one moment they can be read back.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tempfile::TempDir;

use super::steps::{TEMP_PREFIX, ZERO_CHUNK};
use super::{cancel, open, run, wipe_path, Stage, WITNESS};
use crate::models::{App, FileName, Overlay, Panel};
use crate::status::Status;

/// What the seam saw: each stage with the path it was called for, and the
/// bytes on disk at that moment.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    stage: Stage,
    path: PathBuf,
    bytes: Vec<u8>,
}

/// Installs the witness for one test and takes it down with the test, so a
/// failing assertion cannot leave it behind for the next test on this thread.
struct Witness(Rc<RefCell<Vec<Seen>>>);

impl Witness {
    fn install() -> Self {
        let log: Rc<RefCell<Vec<Seen>>> = Rc::default();
        let sink = Rc::clone(&log);
        WITNESS.with(|witness| {
            *witness.borrow_mut() = Some(Box::new(move |stage: Stage, path: &Path| {
                let bytes = fs::read(path).unwrap_or_default();
                sink.borrow_mut().push(Seen {
                    stage,
                    path: path.to_path_buf(),
                    bytes,
                });
            }));
        });
        Self(log)
    }

    fn seen(&self) -> Vec<Seen> {
        self.0.borrow().clone()
    }

    fn at(&self, stage: Stage) -> Vec<Seen> {
        self.seen()
            .into_iter()
            .filter(|seen| seen.stage == stage)
            .collect()
    }
}

impl Drop for Witness {
    fn drop(&mut self) {
        WITNESS.with(|witness| *witness.borrow_mut() = None);
    }
}

fn app_in(dir: &Path) -> App {
    let left = Panel::new(dir.to_path_buf()).expect("left panel");
    let right = Panel::new(dir.to_path_buf()).expect("right panel");
    App::with_panels(left, right)
}

fn status_text(app: &App) -> String {
    app.status.to_string()
}

mod confirmation;
mod special_entries;
mod zero_passes;
