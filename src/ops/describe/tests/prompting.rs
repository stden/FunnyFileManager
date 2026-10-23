//! Purpose: Ctrl+Z's prompt: opening, accepting, clearing, marked entries and failures.

use super::*;

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

#[test]
fn open_offers_the_existing_description_of_the_entry_under_the_cursor() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_over(temp.path());
    open(&mut app);
    assert_eq!(prompt(&app).name(), "a.txt");
    assert_eq!(prompt(&app).text(), "Old");
    assert_eq!(prompt(&app).pending().len(), 0);
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
    assert_eq!(marked(&app).len(), 0, "nothing is left marked");
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
