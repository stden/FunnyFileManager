//! Exercise complete menu workflows against isolated files and configuration.

use super::*;
use crate::{
    menu::{Heading, Menu, SettingsPage},
    models::{
        ChoiceTarget, MenuChoice, MenuDialog, Overlay, Panel, PanelSide, PanelsShown, ViewMode,
    },
};
use std::fs;
use tempfile::TempDir;

fn fixture() -> (TempDir, App) {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("folder")).unwrap();
    fs::write(temp.path().join("alpha.txt"), "alpha\ttext").unwrap();
    fs::write(temp.path().join("beta.bin"), "beta").unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    app.settings =
        crate::models::settings::Settings::with_file(temp.path().join("config/settings.yaml"));
    select(&mut app, "alpha.txt");
    (temp, app)
}

fn select(app: &mut App, name: &str) {
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap();
    app.left_panel.select(Some(index));
}

fn rows(app: &App) -> &[MenuChoice] {
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("expected menu dialog")
    };
    let MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("expected choices")
    };
    rows
}

mod command_rows;
mod finding;
mod panel_rows;
mod prompt_rows;
mod restarts;
