//! Purpose: Keep the bulk Git command available in the F2 and F9 menus.

use super::*;
use crate::menu::{Heading, Menu};

const LABEL: &str = "Git pull for all repositories";

#[test]
fn git_pull_is_available_without_a_user_command_file() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let panel = crate::models::Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    open(&mut app).expect("user menu");
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("user menu must open");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("user menu must offer choices");
    };
    let choice = rows
        .iter()
        .find(|row| row.label == LABEL)
        .expect("bulk Git command")
        .target
        .clone();
    crate::ops::menu::choose(&mut app, choice).expect("choose command");
    assert_eq!(app.git_pull_root.as_deref(), Some(temp.path()));
    assert_eq!(fs::read_dir(temp.path()).expect("directory").count(), 0);
}

#[test]
fn git_pull_is_available_in_the_commands_menu() {
    let menu = Menu::open(Heading::Commands);
    let entry = menu
        .entries()
        .iter()
        .find(|entry| crate::menu::label_of(entry).replace('&', "") == LABEL)
        .expect("Commands must offer bulk Git pull");
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let panel = crate::models::Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    let Some(crate::action::Action::Menu(cmd)) = entry.action() else {
        panic!("expected Action::Menu");
    };
    crate::ops::menu::execute(&mut app, cmd);
    assert_eq!(app.git_pull_root.as_deref(), Some(temp.path()));
}
