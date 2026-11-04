//! Capture metadata for extended panel views outside rendering.

use std::{collections::BTreeMap, fs, io, path::Path};

use crate::models::{DirEntry, Panel, PanelDetails, PanelView};

#[cfg(windows)]
pub mod windows;

pub fn set_view(panel: &mut Panel, view: PanelView) -> io::Result<()> {
    panel.details = details_for(
        &panel.path,
        panel.entries.iter(),
        view,
        panel.active_description_names(),
    )?;
    panel.view = view;
    Ok(())
}

/// Capture view-specific details before mutating the panel snapshot.
pub(super) fn details_for<'a>(
    path: &Path,
    entries: impl IntoIterator<Item = &'a DirEntry>,
    view: PanelView,
    description_names: &[String],
) -> io::Result<PanelDetails> {
    let mut details = PanelDetails::default();
    match view {
        PanelView::Descriptions | PanelView::LongDescriptions => {
            details.descriptions = crate::description::try_read(path, description_names)?
                .into_iter()
                .collect();
        }
        PanelView::Owners => {
            details.owners = collect_owners(path, entries)?;
        }
        PanelView::Links => {
            for entry in entries {
                let entry_path = path.join(entry.disk_name());
                if let Ok(metadata) = fs::symlink_metadata(&entry_path) {
                    details
                        .links
                        .insert(entry.name.clone(), links(&entry_path, &metadata));
                }
            }
        }
        PanelView::Brief
        | PanelView::Medium
        | PanelView::Full
        | PanelView::Wide
        | PanelView::Detailed
        | PanelView::Alternative => {}
    }
    Ok(details)
}

#[cfg(windows)]
fn collect_owners<'a>(
    path: &Path,
    _entries: impl IntoIterator<Item = &'a DirEntry>,
) -> io::Result<BTreeMap<String, String>> {
    windows::owners(path)
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "matches the fallible Windows ACL lookup signature"
)]
fn collect_owners<'a>(
    path: &Path,
    entries: impl IntoIterator<Item = &'a DirEntry>,
) -> io::Result<BTreeMap<String, String>> {
    let mut owners = BTreeMap::new();
    let mut names = crate::directory::owner::Owners::default();
    for entry in entries {
        let entry_path = path.join(entry.disk_name());
        if let Ok(metadata) = fs::symlink_metadata(&entry_path) {
            let owner = names.user_and_group(&metadata).unwrap_or_default();
            owners.insert(entry.name.clone(), owner);
        }
    }
    Ok(owners)
}

fn links(path: &Path, metadata: &fs::Metadata) -> String {
    if let Ok(target) = fs::read_link(path) {
        return format!("→ {}", target.display());
    }
    link_count(path, metadata)
}

#[cfg(unix)]
fn link_count(_path: &Path, metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink().to_string()
}

#[cfg(windows)]
fn link_count(path: &Path, _metadata: &fs::Metadata) -> String {
    windows::link_count(path)
}

#[cfg(test)]
mod tests;
