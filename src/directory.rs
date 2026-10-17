//! Filesystem adapter for complete panel directory snapshots.
//!
//! Metadata, executable and hidden attributes, parent timestamps and available
//! capacity are captured here. Presentation consumes those facts from state.

use std::{collections::HashMap, fs, io, path::Path};

use crate::models::{
    Attributes, DirEntry, DirectoryListing, EntryKind, FileName, FileSize, Modified, PanelMetadata,
};

pub mod owner;

/// Facts read once for the whole directory and consulted for every row: the
/// owner-name cache and the description file's lines.
struct Context {
    owners: owner::Owners,
    descriptions: HashMap<String, String>,
}

/// Load every filesystem-derived fact needed to display a directory.
///
/// `description_names` is the panel's list of description files
/// (`Settings.descriptions.names`), searched in order.
pub fn read(path: &Path, description_names: &[String]) -> io::Result<DirectoryListing> {
    // Open the directory before querying optional facts. Failure must not
    // publish a partial listing or replace the panel's previous metadata.
    let directory = fs::read_dir(path)?;
    let mut context = Context {
        owners: owner::Owners::default(),
        descriptions: crate::description::read(path, description_names),
    };
    let mut entries = Vec::new();
    let mut metadata = PanelMetadata::default();
    for entry in directory.filter_map(Result::ok) {
        let Ok(attributes) = entry.metadata() else {
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
    metadata.dirty_repos = crate::git::dirty_repos(path, &entries);
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
        let resolved = match &target {
            None => Some(attributes),
            Some(found) => found.as_ref().ok(),
        };
        let sized = resolved.unwrap_or(attributes);
        DirEntry {
            executable: resolved.is_some_and(|meta| is_executable(&name, meta)),
            kind: entry_kind(attributes, resolved),
            size: FileSize::from_bytes(sized.len()),
            modified: Modified::from_metadata(attributes),
            created: Modified::from_result(attributes.created()),
            accessed: Modified::from_result(attributes.accessed()),
            links: link_count(attributes),
            owner: self.owners.of(attributes),
            allocated: allocated_bytes(sized),
            attributes: Attributes::from_metadata(attributes),
            description: self.descriptions.remove(&name),
            name,
            on_disk,
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

/// `st_blocks` counts 512-byte units whatever the filesystem's block size;
/// elsewhere the length is the best available answer.
#[cfg(unix)]
fn allocated_bytes(metadata: &fs::Metadata) -> FileSize {
    use std::os::unix::fs::MetadataExt;

    FileSize::from_bytes(metadata.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
fn allocated_bytes(metadata: &fs::Metadata) -> FileSize {
    FileSize::from_bytes(metadata.len())
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
///
/// Where a panel goes when its own folder has been deleted under it, as Far's
/// does: walking up one level is not enough when the parent went too.
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

/// Unix executable mode bits and Windows runnable extensions are distinct rules.
#[cfg(unix)]
fn is_executable(_name: &str, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(name: &str, _metadata: &fs::Metadata) -> bool {
    const RUNNABLE: [&str; 6] = ["exe", "com", "bat", "cmd", "ps1", "msi"];

    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            RUNNABLE
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
}

/// Windows stores a hidden attribute; a Unix leading dot is just a name.
#[cfg(windows)]
fn is_hidden(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
fn is_hidden(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests;
