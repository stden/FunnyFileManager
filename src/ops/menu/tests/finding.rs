//! Purpose: Find file and find folder: results, unreadable folders and the entry limit.

use super::*;

#[test]
fn search_results_open_files_and_navigate_to_folders() {
    let (temp, mut app) = fixture();
    submit(&mut app, MenuCommand::FindFile, "*.txt").unwrap();
    assert_eq!(rows(&app).len(), 1);
    let target = rows(&app)[0].target.clone();
    choose(&mut app, target).unwrap();
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
    assert_eq!(
        app.view_history.last(),
        Some(temp.path().join("alpha.txt").as_path())
    );
    submit(&mut app, MenuCommand::FindFolder, "fold*").unwrap();
    let target = rows(&app)[0].target.clone();
    choose(&mut app, target).unwrap();
    assert_eq!(app.left_panel.path, temp.path().join("folder"));
    execute_result(&mut app, MenuCommand::FolderHistory).unwrap();
    assert_eq!(rows(&app).len(), 1);
    execute_result(&mut app, MenuCommand::ViewHistory).unwrap();
    assert_eq!(rows(&app).len(), 1);
}

#[test]
fn search_keeps_matches_when_a_child_directory_cannot_be_read() {
    let (temp, mut app) = fixture();
    let blocked = temp.path().join("blocked");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("secret.txt"), "secret").unwrap();
    let _deny = search::DenyReads::named("blocked");
    submit(&mut app, MenuCommand::FindFile, "*").unwrap();
    let labels: Vec<_> = rows(&app).iter().map(|row| row.label.clone()).collect();
    assert!(labels.iter().any(|label| label.contains("alpha.txt")));
    assert!(labels.iter().all(|label| !label.contains("secret")));
    assert!(app.status.to_string().contains("could not be read"));
}

#[test]
fn search_says_how_many_unreadable_folders_it_skipped() {
    let (temp, mut app) = fixture();
    fs::create_dir(temp.path().join("blocked")).unwrap();
    fs::create_dir(temp.path().join("folder/blocked")).unwrap();
    let _deny = search::DenyReads::named("blocked");
    submit(&mut app, MenuCommand::FindFolder, "*").unwrap();
    assert_eq!(rows(&app).len(), 3, "folder, blocked and folder/blocked");
    assert_eq!(
        app.status.to_string(),
        "Search skipped 2 folders that could not be read."
    );
}

#[test]
fn search_says_when_the_entry_limit_stops_it() {
    let (_temp, mut app) = fixture();
    app.settings.set(SettingsPage::System, "1").unwrap();
    submit(&mut app, MenuCommand::FindFile, "*").unwrap();
    assert!(rows(&app).len() <= 1);
    assert!(app.status.to_string().contains("entry limit"));
}

#[test]
fn folder_search_respects_depth_and_entry_limits() {
    let (temp, mut app) = fixture();
    fs::create_dir_all(temp.path().join("folder/deep/deeper")).unwrap();
    app.settings.set(SettingsPage::Tree, "1").unwrap();
    submit(&mut app, MenuCommand::FindFolder, "*").unwrap();
    assert_eq!(rows(&app).len(), 2);
    app.settings.set(SettingsPage::System, "1").unwrap();
    submit(&mut app, MenuCommand::FindFolder, "*").unwrap();
    assert!(rows(&app).len() <= 1);
}

/// Alt+F7 with text: a 10.5 MB log with the text on its first line used to be
/// skipped for its size, and nothing said so.
#[test]
fn a_text_search_reads_files_over_ten_megabytes() {
    let (temp, mut app) = fixture();
    let mut text = String::from("needle at top\n");
    text.push_str(&"0000001 INFO nothing to see\n".repeat(400_000));
    fs::write(temp.path().join("big-needle.txt"), text).unwrap();
    fs::write(temp.path().join("z-needle.txt"), "a needle").unwrap();

    find_files(&mut app, "*", "NEEDLE");

    let mut labels: Vec<_> = rows(&app).iter().map(|row| row.label.clone()).collect();
    labels.sort();
    assert_eq!(labels, ["big-needle.txt", "z-needle.txt"]);
}

#[test]
fn search_says_when_the_time_limit_stops_it() {
    let (_temp, mut app) = fixture();
    search::find_within(&mut app, "*", false, "alpha", std::time::Duration::ZERO);
    assert!(rows(&app).is_empty());
    assert_eq!(
        app.status.to_string(),
        "Search stopped at the time limit. The list is incomplete."
    );
}

/// What the progress dialog was asked to draw.
#[derive(Clone, Default)]
struct Recorder(std::rc::Rc<std::cell::RefCell<Vec<crate::progress::Snapshot>>>);

impl crate::progress::Painter for Recorder {
    fn paint(&mut self, snapshot: &crate::progress::Snapshot) {
        self.0.borrow_mut().push(snapshot.clone());
    }
}

/// Alt+F7 used to walk the tree with nothing on screen, so a long search
/// looked like a hang: it now shows the folder it is in and what it found.
#[test]
fn a_search_shows_its_progress_while_it_walks() {
    let (temp, mut app) = fixture();
    fs::write(temp.path().join("folder/gamma.txt"), "gamma").unwrap();
    let recorder = Recorder::default();
    let _installed =
        crate::progress::install_with(Box::new(recorder.clone()), std::time::Duration::ZERO);

    find_files(&mut app, "*.txt", "");

    let frames = recorder.0.borrow().clone();
    assert!(!frames.is_empty(), "the search drew nothing");
    assert!(frames
        .iter()
        .all(|frame| frame.operation == crate::progress::Operation::Find));
    assert!(frames
        .iter()
        .any(|frame| frame.current == temp.path().join("folder")));
    let last = frames.last().unwrap();
    assert_eq!(last.done.files, 2, "alpha.txt and folder/gamma.txt");
    assert_eq!(last.total.files, 4, "every entry looked at");
    assert!(
        !crate::progress::is_shown(),
        "the dialog ends with the search"
    );
    assert!(crate::progress::take_painted());
}
