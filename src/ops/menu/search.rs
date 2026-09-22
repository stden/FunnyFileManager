//! Bounded filesystem search and directory comparisons for menu commands.

use crate::{
    mask::Mask,
    menu::MenuCommand,
    models::{App, ChoiceTarget, MenuChoice},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn find(app: &mut App, pattern: &str, folders: bool) -> io::Result<()> {
    let root = app.get_active_panel_ref().path.clone();
    let mut rows = Vec::new();
    let mut budget = app.settings.number(crate::menu::SettingsPage::System);
    walk(
        &root,
        app.settings.number(crate::menu::SettingsPage::Tree),
        &Mask::parse(pattern),
        folders,
        &mut rows,
        &mut budget,
    )?;
    super::choices(app, if folders { "Find folder" } else { "Find file" }, rows);
    Ok(())
}

fn walk(
    root: &Path,
    depth_limit: usize,
    mask: &Mask,
    folders: bool,
    rows: &mut Vec<MenuChoice>,
    budget: &mut usize,
) -> io::Result<()> {
    let mut pending = vec![(root.to_path_buf(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        if *budget == 0 {
            break;
        }
        for entry in fs::read_dir(directory)?.flatten() {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() && depth < depth_limit {
                pending.push((path.clone(), depth + 1));
            }
            if kind.is_dir() == folders && mask.matches(&entry.file_name().to_string_lossy()) {
                add_result(root, path, folders, rows);
            }
        }
    }
    Ok(())
}

fn add_result(root: &Path, path: PathBuf, folders: bool, rows: &mut Vec<MenuChoice>) {
    let label = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string();
    let target = if folders {
        ChoiceTarget::Directory(path)
    } else {
        ChoiceTarget::File(path)
    };
    rows.push(MenuChoice { label, target });
}

pub(super) fn compare(app: &mut App) {
    let (left, right) = (&mut app.left_panel, &mut app.right_panel);
    left.marked.clear();
    right.marked.clear();
    mark_different(left, right);
    mark_different(right, left);
    app.set_status(crate::status::Status::CommandOutput(
        "Marked missing or different-sized files in both panels".to_owned(),
    ));
}

fn mark_different(panel: &mut crate::models::Panel, other: &crate::models::Panel) {
    for entry in &panel.entries {
        if !entry.kind.is_dir()
            && !other
                .entries
                .iter()
                .any(|candidate| candidate.name == entry.name && candidate.size == entry.size)
        {
            panel
                .marked
                .insert(crate::models::FileName::from(entry.name.as_str()));
        }
    }
}

pub(super) fn prompt(app: &mut App, command: MenuCommand) {
    let folders = matches!(command, MenuCommand::FindFolder);
    super::prompt(
        app,
        command,
        if folders { "Find folder" } else { "Find file" },
        "Name mask (recursive, maximum 100,000 entries)",
        "*",
    );
}
