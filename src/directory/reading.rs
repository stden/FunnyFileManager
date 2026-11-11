//! Purpose: Interruptible directory reading with top-border progress updates.

use std::ffi::OsString;
use std::{fs, io, path::Path};

use super::{
    apply_descriptions, decode_file_name, entry_metadata, note_sidecar, read_parent,
    record_if_hidden, Context,
};
use crate::models::{DirectoryListing, PanelMetadata};
use crate::progress::{begin_directory_read, PanelHeader};
use crate::values::entry::{DirEntry, FileSize};

/// A directory snapshot and whether `cancel` stopped the walk early.
#[derive(Debug)]
pub struct ReadOutcome {
    pub listing: DirectoryListing,
    pub interrupted: bool,
}

#[derive(Default)]
struct Walk {
    context: Context,
    entries: Vec<DirEntry>,
    sidecars: Vec<OsString>,
    metadata: PanelMetadata,
}

impl Walk {
    fn collect(
        &mut self,
        directory: fs::ReadDir,
        description_names: &[String],
        header: Option<PanelHeader>,
        mut cancel: impl FnMut(bool) -> bool,
    ) -> bool {
        let mut progress = begin_directory_read(header);
        for entry in directory.filter_map(Result::ok) {
            let (name, on_disk) = decode_file_name(entry.file_name());
            note_sidecar(
                &name,
                on_disk.as_deref(),
                description_names,
                &mut self.sidecars,
            );
            let Ok(attributes) = entry_metadata(&entry) else {
                continue;
            };
            let row = self.context.entry(&entry, &attributes, (name, on_disk));
            record_if_hidden(&attributes, &row.name, &mut self.metadata);
            self.entries.push(row);
            let shown = progress.step();
            if cancel(shown) {
                return true;
            }
        }
        false
    }
}

/// Read `path` for a panel at `header`, stopping early when `cancel` asks to.
pub fn read_for_panel(
    path: &Path,
    description_names: &[String],
    header: Option<PanelHeader>,
    cancel: impl FnMut(bool) -> bool,
) -> io::Result<ReadOutcome> {
    // Open the directory before starting progress or querying optional facts.
    // Failure must not publish a partial listing or paint a banner.
    let directory = fs::read_dir(path)?;
    let mut walk = Walk::default();
    let interrupted = walk.collect(directory, description_names, header, cancel);
    if !interrupted {
        apply_descriptions(path, description_names, &walk.sidecars, &mut walk.entries);
        crate::git::load_repo_facts(path, &walk.entries, &mut walk.metadata);
    }
    let parent = read_parent(path, &mut walk.metadata);
    walk.metadata.available_space =
        crate::diskfree::available_bytes(path).map(FileSize::from_bytes);
    Ok(ReadOutcome {
        listing: DirectoryListing {
            entries: walk.entries,
            parent,
            metadata: walk.metadata,
        },
        interrupted,
    })
}
