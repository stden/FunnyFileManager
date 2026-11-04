//! Filesystem adapter for complete panel directory snapshots.
//!
//! Metadata, executable and hidden attributes, parent timestamps and available
//! capacity are captured here. Presentation consumes those facts from state.

use std::ffi::{OsStr, OsString};
use std::{fs, io, path::Path};

use crate::models::{DirectoryListing, PanelMetadata};
use crate::values::entry::{Attributes, DirEntry, EntryKind, FileSize, Modified};

pub mod owner;
mod platform;

pub use platform::link_count;
use platform::{allocated_bytes, is_executable, record_if_hidden};

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
    let mut sidecars = Vec::new();
    let mut metadata = PanelMetadata::default();
    for entry in directory.filter_map(Result::ok) {
        let (name, on_disk) = decode_file_name(entry.file_name());
        note_sidecar(&name, on_disk.as_deref(), description_names, &mut sidecars);
        let Ok(attributes) = entry_metadata(&entry) else {
            continue;
        };
        let row = context.entry(&entry, &attributes, (name, on_disk));
        record_if_hidden(&attributes, &row.name, &mut metadata);
        entries.push(row);
    }
    apply_descriptions(path, description_names, &sidecars, &mut entries);
    crate::git::load_repo_facts(path, &entries, &mut metadata);
    let parent = read_parent(path, &mut metadata);
    metadata.available_space = crate::diskfree::available_bytes(path).map(FileSize::from_bytes);
    Ok(DirectoryListing {
        entries,
        parent,
        metadata,
    })
}

fn decode_file_name(disk_name: OsString) -> (String, Option<OsString>) {
    match disk_name.into_string() {
        Ok(name) => (name, None),
        Err(raw) => (raw.to_string_lossy().into_owned(), Some(raw)),
    }
}

fn note_sidecar(
    name: &str,
    on_disk: Option<&OsStr>,
    description_names: &[String],
    sidecars: &mut Vec<OsString>,
) {
    if description_names
        .iter()
        .any(|wanted| crate::casefold::eq_ignore_case(name, wanted))
    {
        let fallback = OsStr::new(name);
        sidecars.push(OsString::from(on_disk.unwrap_or(fallback)));
    }
}

fn apply_descriptions(
    path: &Path,
    description_names: &[String],
    sidecars: &[OsString],
    entries: &mut [DirEntry],
) {
    if sidecars.is_empty() {
        return;
    }
    let mut descriptions = crate::description::read_listed(path, description_names, sidecars);
    for row in entries {
        row.description = descriptions.remove(&row.name);
    }
}

fn symlink_target_metadata(entry: &fs::DirEntry, meta: &fs::Metadata) -> Option<fs::Metadata> {
    if meta.is_symlink() {
        fs::metadata(entry.path()).ok()
    } else {
        None
    }
}

impl Context {
    /// One row from the metadata `read_dir` already fetched — nothing here
    /// touches the disk again except the symlink-target probes.
    fn entry(
        &mut self,
        entry: &fs::DirEntry,
        attributes: &fs::Metadata,
        (name, on_disk): (String, Option<OsString>),
    ) -> DirEntry {
        let target_meta = symlink_target_metadata(entry, attributes);
        let effective = if attributes.is_symlink() {
            target_meta.as_ref()
        } else {
            Some(attributes)
        };
        let sized = effective.unwrap_or(attributes);
        let kind = entry_kind(attributes, effective);
        let (size, allocated) = if kind.is_dir() {
            (FileSize::default(), FileSize::default())
        } else {
            (FileSize::from_bytes(sized.len()), allocated_bytes(sized))
        };
        DirEntry {
            executable: effective.is_some_and(|meta| is_executable(&name, meta)),
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

fn read_parent(path: &Path, metadata: &mut PanelMetadata) -> Option<DirEntry> {
    path.parent()?;
    let attributes = fs::metadata(path.join(".."));
    if let Ok(meta) = &attributes {
        record_if_hidden(meta, "..", metadata);
    }
    Some(DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::Directory,
        // The synthetic row is dated from the directory it leads to.
        modified: attributes.map_or(Modified::UNKNOWN, |meta| Modified::from_metadata(&meta)),
        ..DirEntry::default()
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
