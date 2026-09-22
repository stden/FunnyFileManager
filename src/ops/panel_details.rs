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
        for entry in &panel.entries {
            let path = panel.path.join(&entry.name);
            if let Ok(metadata) = fs::symlink_metadata(&path) {
                details.owners.insert(entry.name.clone(), owner(&metadata));
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

fn panel_descriptions(panel: &Panel) -> io::Result<std::collections::BTreeMap<String, String>> {
    if panel.options.description_file.is_empty() || panel.options.description_file == "descript.ion"
    {
        descriptions(&panel.path)
    } else {
        descriptions_in(&panel.path, &panel.options.description_file)
    }
}

pub fn descriptions(directory: &Path) -> io::Result<std::collections::BTreeMap<String, String>> {
    descriptions_in(directory, "descript.ion")
}

pub fn descriptions_in(
    directory: &Path,
    filename: &str,
) -> io::Result<std::collections::BTreeMap<String, String>> {
    let path = directory.join(filename);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    Ok(text.lines().filter_map(description).collect())
}

fn description(line: &str) -> Option<(String, String)> {
    let (name, description) = if let Some(quoted) = line.strip_prefix('"') {
        quoted.split_once('"')?
    } else {
        line.split_once(char::is_whitespace)?
    };
    Some((name.to_owned(), description.trim_start().to_owned()))
}

#[cfg(unix)]
fn owner(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("{}:{}", metadata.uid(), metadata.gid())
}

#[cfg(not(unix))]
fn owner(_metadata: &fs::Metadata) -> String {
    String::new()
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
