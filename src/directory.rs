//! Filesystem adapter for complete panel directory snapshots.
//!
//! Metadata, executable and hidden attributes, parent timestamps and available
//! capacity are captured here. Presentation consumes those facts from state.

use std::{fs, io, path::Path};

use crate::models::{DirectoryListing, FileName, PanelMetadata};
use crate::values::entry::{Attributes, DirEntry, EntryKind, FileSize, Modified};

pub mod owner;
mod platform;

use platform::{allocated_bytes, is_executable, is_hidden};

#[cfg(test)]
thread_local! {
    static FORCE_ENTRY_METADATA_ERROR: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

fn entry_metadata(entry: &fs::DirEntry) -> io::Result<fs::Metadata> {
    #[cfg(test)]
    if FORCE_ENTRY_METADATA_ERROR.get() {
        return Err(io::Error::other("forced entry metadata error"));
    }
    entry.metadata()
}

/// Facts read once for the whole directory and consulted for every row: the
/// owner-name cache.
#[derive(Default)]
struct Context {
    owners: owner::Owners,
}

/// Load every filesystem-derived fact needed to display a directory.
///
/// `description_names` is the panel's list of description files
/// (`Settings.descriptions.names`), searched in order.
pub fn read(path: &Path, description_names: &[String]) -> io::Result<DirectoryListing> {
    // Open the directory before querying optional facts. Failure must not
    // publish a partial listing or replace the panel's previous metadata.
    let directory = fs::read_dir(path)?;
    let mut context = Context::default();
    let mut entries = Vec::new();
    let mut listed_names = Vec::new();
    let mut metadata = PanelMetadata::default();
    for entry in directory.filter_map(Result::ok) {
        listed_names.push(entry.file_name());
        let Ok(attributes) = entry_metadata(&entry) else {
            continue;
        };
        let row = context.entry(&entry, &attributes);
        if is_hidden(&attributes) {
            metadata
                .hidden_names
                .insert(FileName::new(row.name.clone()));
        }
        entries.push(row);
    }
    let mut descriptions = crate::description::read_listed(path, description_names, &listed_names);
    for row in &mut entries {
        row.description = descriptions.remove(&row.name);
    }
    crate::git::load_repo_facts(path, &entries, &mut metadata);
    let parent = read_parent(path, &mut metadata);
    metadata.available_space = crate::diskfree::available_bytes(path).map(FileSize::from_bytes);
    Ok(DirectoryListing {
        entries,
        parent,
        metadata,
    })
}

impl Context {
    /// One row from the metadata `read_dir` already fetched — nothing here
    /// touches the disk again except the symlink-target probes.
    fn entry(&mut self, entry: &fs::DirEntry, attributes: &fs::Metadata) -> DirEntry {
        let disk_name = entry.file_name();
        let name = disk_name.to_string_lossy().into_owned();
        let on_disk = disk_name.to_str().is_none().then_some(disk_name);
        let target = attributes.is_symlink().then(|| fs::metadata(entry.path()));
        let resolved = target
            .as_ref()
            .map_or(Some(attributes), |found| found.as_ref().ok());
        let sized = resolved.unwrap_or(attributes);
        let kind = entry_kind(attributes, resolved);
        let (size, allocated) = if kind.is_dir() {
            (FileSize::default(), FileSize::default())
        } else {
            (FileSize::from_bytes(sized.len()), allocated_bytes(sized))
        };
        DirEntry {
            executable: resolved.is_some_and(|meta| is_executable(&name, meta)),
            kind,
            size,
            modified: Modified::from_metadata(attributes),
            created: Modified::from_result(attributes.created()),
            accessed: Modified::from_result(attributes.accessed()),
            links: link_count(attributes),
            owner: self.owners.of(attributes),
            allocated,
            attributes: Attributes::from_metadata(attributes),
            description: None,
            name,
            on_disk,
            directory_size: crate::values::entry::DirectorySize::Unmeasured,
        }
    }
}

/// `st_nlink`; other platforms report every entry as singly linked.
#[cfg(unix)]
pub fn link_count(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;

    u32::try_from(metadata.nlink()).unwrap_or(u32::MAX)
}

#[cfg(not(unix))]
pub fn link_count(_metadata: &fs::Metadata) -> u32 {
    1
}

fn read_parent(path: &Path, metadata: &mut PanelMetadata) -> Option<DirEntry> {
    path.parent().map(|_| {
        let attributes = fs::metadata(path.join(".."));
        if attributes.as_ref().is_ok_and(is_hidden) {
            metadata.hidden_names.insert(FileName::from(".."));
        }
        DirEntry {
            name: "..".to_owned(),
            kind: EntryKind::Directory,
            // The synthetic row is dated from the directory it leads to.
            modified: attributes.map_or(Modified::UNKNOWN, |meta| Modified::from_metadata(&meta)),
            ..DirEntry::default()
        }
    })
}

/// The closest folder at or above `path` that still exists.
pub fn nearest_existing(path: &Path) -> Option<&Path> {
    path.ancestors().find(|folder| folder.is_dir())
}

/// A symlink behaves like its `target` while retaining its identity as a
/// link; a `target` of `None` is a link with nothing behind it.
fn entry_kind(metadata: &fs::Metadata, target: Option<&fs::Metadata>) -> EntryKind {
    if metadata.is_symlink() {
        match target {
            Some(target) if target.is_dir() => EntryKind::SymlinkToDirectory,
            Some(_) => EntryKind::SymlinkToFile,
            None => EntryKind::BrokenSymlink,
        }
    } else if metadata.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}

#[cfg(test)]
mod tests;
