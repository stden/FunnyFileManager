//! Purpose: Which description file is written, and the cursor after the relist.

use super::*;

fn type_text(app: &mut App, text: &str) {
    let Overlay::Describe(prompt) = &mut app.overlay else {
        panic!("no prompt");
    };
    for ch in text.chars() {
        prompt.push(ch);
    }
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

/// The names in `dir`, spelled as the file system stores them.
fn file_names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
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
    // Names, not `exists()`: on a case-insensitive file system (Windows,
    // macOS) `Descript.ion` answers for `descript.ion`.
    let names = file_names(temp.path());
    assert!(names.contains(&"descript.ion".to_owned()), "{names:?}");
    assert!(!names.contains(&"Descript.ion".to_owned()), "{names:?}");
    let shown = &app.get_active_panel_ref().details.descriptions["a.txt"];
    assert_eq!(shown, "Old and new", "the view shows the edit");
    assert_eq!(described(&app, "a.txt").as_deref(), Some("Old and new"));
}
