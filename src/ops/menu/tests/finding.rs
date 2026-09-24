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
