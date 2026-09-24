//! Capture metadata for extended panel views outside rendering.

use std::{fs, io, path::Path};

use crate::models::{Panel, PanelDetails, PanelView};

#[cfg(windows)]
mod windows;

pub fn set_view(panel: &mut Panel, view: PanelView) -> io::Result<()> {
    let mut details = PanelDetails::default();
    if matches!(view, PanelView::Descriptions | PanelView::LongDescriptions) {
        details.descriptions = panel_descriptions(panel)?;
    }
    if matches!(view, PanelView::Owners | PanelView::Links) {
        let mut names = crate::directory::owner::Owners::default();
        for entry in &panel.entries {
            let path = panel.path.join(entry.disk_name());
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                let owner = names.user_and_group(&metadata).unwrap_or_default();
                details.owners.insert(entry.name.clone(), owner);
                details
                    .links
                    .insert(entry.name.clone(), links(&path, &metadata));
            }
        }
    }
    #[cfg(windows)]
    if view == PanelView::Owners {
        details.owners = windows::owners(&panel.path)?;
    }
    panel.details = details;
    panel.view = view;
    Ok(())
}

/// The Descriptions views read the file named in Options > File descriptions,
/// or the panel's own search list, by the same rule as the listing and
/// Ctrl+Z.
fn panel_descriptions(panel: &Panel) -> io::Result<std::collections::BTreeMap<String, String>> {
    let chosen = &panel.options.description_file;
    let names = if chosen.is_empty() {
        panel.description_names.as_slice()
    } else {
        std::slice::from_ref(chosen)
    };
    Ok(crate::description::try_read(&panel.path, names)?
        .into_iter()
        .collect())
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
