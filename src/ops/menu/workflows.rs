//! History, filtering, screen selection and host information commands.

use crate::{
    menu::MenuCommand,
    models::{App, ChoiceTarget, MenuChoice, Overlay, ViewMode, Viewer},
};
use std::{io, process::Command};

pub(super) fn execute(app: &mut App, command: MenuCommand) -> io::Result<()> {
    match command {
        MenuCommand::Filter => {
            open_filter(app);
        }
        MenuCommand::LongNames => crate::ops::longnames::toggle(app),
        MenuCommand::TogglePanel(side) => crate::ops::panels::toggle(app, side),
        MenuCommand::FolderHistory => folder_history(app),
        MenuCommand::ViewHistory => view_history(app),
        MenuCommand::Screens => screens(app),
        MenuCommand::Tasks => tasks(app)?,
        MenuCommand::Devices => devices(app),
        MenuCommand::VideoMode => crate::terminal::toggle_video_mode()?,
        _ => return Err(io::Error::other("Not a navigation command")),
    }
    Ok(())
}

fn open_filter(app: &mut App) {
    let filter = app.get_active_panel_ref().options.filter.clone();
    super::prompt(
        app,
        MenuCommand::Filter,
        "File panel filter",
        "Filename mask; empty shows all files. Directories remain visible.",
        &filter,
    );
}

pub(super) fn filter(app: &mut App, text: &str) -> io::Result<()> {
    let panel = app.get_active_panel();
    let previous = std::mem::replace(&mut panel.options.filter, text.to_owned());
    if let Err(error) = panel.load_entries() {
        panel.options.filter = previous;
        return Err(error);
    }
    app.close_overlay();
    Ok(())
}

fn folder_history(app: &mut App) {
    let rows = app
        .get_active_panel_ref()
        .navigation_history
        .iter()
        .rev()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::Directory(path.clone()),
        })
        .collect();
    super::choices(app, "Folders history", rows);
}

fn view_history(app: &mut App) {
    let rows = app
        .viewed_files
        .iter()
        .rev()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::File(path.clone()),
        })
        .collect();
    super::choices(app, "File view history", rows);
}

fn screens(app: &mut App) {
    super::choices(
        app,
        "Screens",
        vec![
            MenuChoice {
                label: "File panels".to_owned(),
                target: ChoiceTarget::Screen(ViewMode::Panels),
            },
            MenuChoice {
                label: "Console transcript".to_owned(),
                target: ChoiceTarget::Screen(ViewMode::Console),
            },
        ],
    );
}

fn devices(app: &mut App) {
    let rows = App::get_drives()
        .into_iter()
        .map(|path| MenuChoice {
            label: path.display().to_string(),
            target: ChoiceTarget::Directory(path),
        })
        .collect();
    super::choices(app, "Mounted devices", rows);
}

fn tasks(app: &mut App) -> io::Result<()> {
    let mut command = if cfg!(windows) {
        Command::new("tasklist")
    } else {
        Command::new("ps")
    };
    if !cfg!(windows) {
        command.args(["-eo", "pid,comm,%cpu,%mem"]);
    }
    let output = super::process::capture(&mut command)?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    app.set_overlay(Overlay::Viewer(Viewer::report("Task list", lines)));
    Ok(())
}
