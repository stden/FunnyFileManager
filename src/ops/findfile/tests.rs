use super::{accept, open};
use crate::models::{App, Overlay, Panel};
use std::fs;
use tempfile::TempDir;

fn app_with(files: &[&str]) -> (TempDir, App) {
    let dir = TempDir::new().expect("dir");
    for name in files {
        fs::write(dir.path().join(name), name).expect("file");
    }
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    (dir, App::from_panels(panel.clone(), panel))
}

#[test]
fn open_puts_up_the_find_file_overlay() {
    let (_dir, mut app) = app_with(&["a.txt"]);
    open(&mut app);
    assert!(matches!(app.overlay, Overlay::FindFile(_)));
}

#[test]
fn accept_lists_matching_files() {
    let (_dir, mut app) = app_with(&["a.txt", "b.bin"]);
    open(&mut app);
    accept(&mut app);
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("results list");
    };
    let crate::models::MenuDialog::Choices { title, rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(title, "Find file");
    assert_eq!(rows.len(), 2);
}

#[test]
fn accept_treats_a_blank_mask_as_star() {
    let (_dir, mut app) = app_with(&["a.txt"]);
    open(&mut app);
    if let Overlay::FindFile(dialog) = &mut app.overlay {
        dialog.mask.clear();
    }
    accept(&mut app);
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("results list");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(rows.len(), 1);
}

#[test]
fn find_files_keeps_only_files_whose_text_contains_the_query() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("yes.txt"), "alpha TODO beta").expect("file");
    fs::write(dir.path().join("no.txt"), "nothing here").expect("file");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    crate::ops::menu::find_files(&mut app, "*", "todo");
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("results list");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("yes.txt"), "{:?}", rows[0].label);
}

#[test]
fn accept_keeps_only_files_whose_text_contains_the_query() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("yes.txt"), "alpha TODO beta").expect("file");
    fs::write(dir.path().join("no.txt"), "nothing here").expect("file");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    open(&mut app);
    if let Overlay::FindFile(dialog) = &mut app.overlay {
        dialog.containing = "todo".to_owned();
    }
    accept(&mut app);
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("results list");
    };
    let crate::models::MenuDialog::Choices { rows, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("yes.txt"), "{:?}", rows[0].label);
}

#[test]
fn accept_without_the_dialog_does_nothing() {
    let (_dir, mut app) = app_with(&["a.txt"]);
    accept(&mut app);
    assert!(!app.overlay.is_active());
}
