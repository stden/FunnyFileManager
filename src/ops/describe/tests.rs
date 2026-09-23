//! Unit tests: Ctrl+Z reads the description under the cursor, writes Far's
//! line, re-lists the panel, and walks the marked entries in turn.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{accept, open};
use crate::models::{App, FileName, Overlay, Panel};
use crate::status::Status;

/// Two files, one with a description already, and the panel standing on `a.txt`.
fn app_over(dir: &Path) -> App {
    fs::write(dir.join("a.txt"), "").expect("a.txt");
    fs::write(dir.join("b c.txt"), "").expect("b c.txt");
    fs::write(dir.join("Descript.ion"), "a.txt Old\n").expect("Descript.ion");
    let mut app = App::with_panels(
        Panel::new(dir.to_path_buf()).expect("left"),
        Panel::new(dir.to_path_buf()).expect("right"),
    );
    assert!(app.get_active_panel().select_named("a.txt"));
    app
}

fn prompt(app: &App) -> &crate::models::DescribePrompt {
    match &app.overlay {
        Overlay::Describe(prompt) => prompt,
        _ => panic!("expected the describe prompt to be open"),
    }
}

fn description_file(dir: &Path) -> String {
    fs::read_to_string(dir.join("Descript.ion")).unwrap_or_default()
}

fn marked(app: &App) -> Vec<&str> {
    app.get_active_panel_ref()
        .marked
        .iter()
        .map(FileName::as_str)
        .collect()
}

fn type_text(app: &mut App, text: &str) {
    let Overlay::Describe(prompt) = &mut app.overlay else {
        panic!("no prompt");
    };
    for ch in text.chars() {
        prompt.push(ch);
    }
}

fn described(app: &App, name: &str) -> Option<String> {
    app.get_active_panel_ref()
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.description.clone())
}

#[test]
fn open_offers_the_existing_description_of_the_entry_under_the_cursor() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    open(&mut app);
    assert_eq!(prompt(&app).name(), "a.txt");
    assert_eq!(prompt(&app).text(), "Old");
    assert!(prompt(&app).pending().is_empty());
}

#[test]
fn open_with_nothing_under_the_cursor_says_so() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    assert!(app.get_active_panel().select_named(".."));
    open(&mut app);
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::CommandOutput(text) if text.contains("Nothing to describe")),
        "{:?}",
        app.status
    );
}

#[test]
fn accept_writes_the_line_relists_and_reports() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    open(&mut app);
    let Overlay::Describe(prompt) = &mut app.overlay else {
        panic!("no prompt");
    };
    prompt.backspace();
    prompt.backspace();
    prompt.backspace();
    for ch in "My notes".chars() {
        prompt.push(ch);
    }
    accept(&mut app);
    assert!(!app.overlay.is_active());
    assert_eq!(description_file(temp.path()), "a.txt My notes\n");
    let entry = app
        .get_active_panel()
        .entries
        .iter()
        .find(|entry| entry.name == "a.txt")
        .expect("a.txt listed");
    assert_eq!(entry.description.as_deref(), Some("My notes"));
    assert_eq!(
        app.status,
        Status::CommandOutput("Described a.txt".to_owned())
    );
}

#[test]
fn accept_with_an_empty_text_removes_the_line() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    open(&mut app);
    let Overlay::Describe(prompt) = &mut app.overlay else {
        panic!("no prompt");
    };
    prompt.backspace();
    prompt.backspace();
    prompt.backspace();
    accept(&mut app);
    assert_eq!(description_file(temp.path()), "");
    assert_eq!(
        app.status,
        Status::CommandOutput("Description of a.txt removed".to_owned())
    );
}

#[test]
fn marked_entries_are_described_one_after_another() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    let panel = app.get_active_panel();
    panel.marked.insert(FileName::new("a.txt"));
    panel.marked.insert(FileName::new("b c.txt"));
    open(&mut app);
    assert_eq!(prompt(&app).name(), "a.txt");
    assert_eq!(prompt(&app).pending(), ["b c.txt".to_owned()]);

    accept(&mut app);
    assert_eq!(prompt(&app).name(), "b c.txt", "the queue moves on");
    assert_eq!(prompt(&app).text(), "", "no description yet");
    assert_eq!(
        marked(&app),
        ["b c.txt"],
        "Far unmarks each file as it is described"
    );
    let Overlay::Describe(prompt) = &mut app.overlay else {
        panic!("no prompt");
    };
    for ch in "Spaced".chars() {
        prompt.push(ch);
    }
    accept(&mut app);
    assert!(!app.overlay.is_active(), "the last one closes the dialog");
    assert!(marked(&app).is_empty(), "nothing is left marked");
    assert_eq!(
        description_file(temp.path()),
        "a.txt Old\n\"b c.txt\" Spaced\n",
        "a name with a space is quoted, as Far writes it"
    );
}

#[test]
fn a_write_failure_closes_the_dialog_and_reports_the_error() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    open(&mut app);
    fs::remove_file(temp.path().join("Descript.ion")).expect("remove");
    fs::create_dir(temp.path().join("Descript.ion")).expect("dir in the way");
    accept(&mut app);
    assert!(!app.overlay.is_active());
    assert!(
        matches!(&app.status, Status::Error(text) if text.contains("a.txt")),
        "{:?}",
        app.status
    );
}

#[test]
fn accept_without_the_prompt_open_does_nothing() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    accept(&mut app);
    assert_eq!(description_file(temp.path()), "a.txt Old\n");
}

/// The file written is the first of the panel's own `description_names`,
/// which is where `Settings.descriptions.names` lands — not a hard-coded
/// `Descript.ion`.
#[test]
fn accept_creates_the_panels_first_description_file_name() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "").expect("a.txt");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("left"),
        Panel::new(temp.path().to_path_buf()).expect("right"),
    );
    let panel = app.get_active_panel();
    panel.description_names = vec!["Notes.txt".to_owned(), "Files.bbs".to_owned()];
    assert!(panel.select_named("a.txt"));
    open(&mut app);
    type_text(&mut app, "Custom");
    accept(&mut app);
    assert_eq!(
        fs::read_to_string(temp.path().join("Notes.txt")).expect("Notes.txt"),
        "a.txt Custom\n"
    );
    assert!(!temp.path().join("Descript.ion").exists());
    assert_eq!(described(&app, "a.txt").as_deref(), Some("Custom"));
}

/// A directory that already keeps its descriptions in `Files.bbs` gets the
/// new line there, not in a second file.
#[test]
fn accept_writes_into_the_description_file_the_directory_already_has() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "").expect("a.txt");
    fs::write(temp.path().join("Files.bbs"), "other.txt Kept\n").expect("Files.bbs");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("left"),
        Panel::new(temp.path().to_path_buf()).expect("right"),
    );
    assert!(app.get_active_panel().select_named("a.txt"));
    open(&mut app);
    type_text(&mut app, "Second");
    accept(&mut app);
    assert_eq!(
        fs::read_to_string(temp.path().join("Files.bbs")).expect("Files.bbs"),
        "other.txt Kept\na.txt Second\n"
    );
    assert!(!temp.path().join("Descript.ion").exists());
}

/// The re-list that creates a brand-new `Descript.ion` shifts the rows below
/// it; the cursor follows the described entry by name.
#[test]
fn the_cursor_stays_on_the_described_entry_after_the_relist() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "").expect("a.txt");
    fs::write(temp.path().join("z.txt"), "").expect("z.txt");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("left"),
        Panel::new(temp.path().to_path_buf()).expect("right"),
    );
    assert!(app.get_active_panel().select_named("z.txt"));
    open(&mut app);
    type_text(&mut app, "Last");
    accept(&mut app);
    let panel = app.get_active_panel_ref();
    assert!(
        panel
            .entries
            .iter()
            .any(|entry| entry.name == "Descript.ion"),
        "the new file is listed"
    );
    assert_eq!(
        panel.get_selected_entry().map(|entry| entry.name.as_str()),
        Some("z.txt")
    );
}

/// A directory that keeps its descriptions in a lowercase `descript.ion`, as
/// Far writes it: Ctrl+Z offers the line that is already there, Enter rewrites
/// that same file, and the Descriptions view shows the new text.
#[test]
fn ctrl_z_edits_a_lowercase_descript_ion_in_place() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("a.txt"), "").expect("a.txt");
    fs::write(temp.path().join("descript.ion"), "a.txt Old\n").expect("descript.ion");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_path_buf()).expect("left"),
        Panel::new(temp.path().to_path_buf()).expect("right"),
    );
    let view = crate::models::PanelView::Descriptions;
    crate::ops::panel_details::set_view(app.get_active_panel(), view).expect("view");
    assert!(app.get_active_panel().select_named("a.txt"));

    open(&mut app);
    assert_eq!(prompt(&app).text(), "Old");
    type_text(&mut app, " and new");
    accept(&mut app);

    assert_eq!(
        fs::read_to_string(temp.path().join("descript.ion")).expect("descript.ion"),
        "a.txt Old and new\n"
    );
    assert!(!temp.path().join("Descript.ion").exists());
    let shown = &app.get_active_panel_ref().details.descriptions["a.txt"];
    assert_eq!(shown, "Old and new", "the view shows the edit");
    assert_eq!(described(&app, "a.txt").as_deref(), Some("Old and new"));
}
