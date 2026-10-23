//! Purpose: Enter in the Extract files dialog: every marked archive, typed destinations and failures.

use super::*;

fn console_text(app: &App) -> String {
    app.console
        .visible(1000)
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `l/` holding two archives and a note, all marked; `r/` empty and inactive.
fn two_marked_archives(temp: &Path) -> (App, PathBuf) {
    let (left, right) = (temp.join("l"), temp.join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    pack(&left, "a.tar.gz");
    // A different member: the two unpack side by side, neither replacing
    // anything the other wrote.
    pack_files(&left, "b.tar", &[("payload/other.txt", "other")]);
    fs::write(left.join("notes.txt"), "x").expect("file");
    let mut app = app_between(&left, &right);
    for name in ["a.tar.gz", "b.tar", "notes.txt"] {
        select(&mut app, name);
        app.get_active_panel().toggle_mark_selected();
    }
    (app, right)
}

#[test]
fn run_unpacks_every_marked_archive_reports_and_clears_the_marks() {
    let temp = TempDir::new().expect("temp");
    let (mut app, right) = two_marked_archives(temp.path());

    open_extract(&mut app);
    run_extract(&mut app);

    assert!(!app.overlay.is_active());
    assert_eq!(
        fs::read_to_string(right.join("payload/nested/hello.txt")).expect("unpacked"),
        "hello"
    );
    assert_eq!(
        app.status,
        Status::Extracted {
            count: 2,
            destination: right,
        }
    );
    assert_eq!(app.get_active_panel().marked.len(), 0, "marks are spent");
    assert!(
        console_text(&app).contains("tar -xf"),
        "{}",
        console_text(&app)
    );
    assert!(
        app.right_panel
            .entries
            .iter()
            .any(|entry| entry.name == "payload"),
        "the other panel was refreshed"
    );
}

#[test]
fn run_creates_a_typed_destination_relative_to_the_active_panel() {
    let temp = TempDir::new().expect("temp");
    pack(temp.path(), "a.tar.gz");
    let mut app = app_between(temp.path(), temp.path());
    select(&mut app, "a.tar.gz");
    open_extract(&mut app);
    if let Overlay::ExtractPrompt(prompt) = &mut app.overlay {
        prompt.destination.set_text("unpacked/here");
    }
    run_extract(&mut app);
    assert!(temp
        .path()
        .join("unpacked/here/payload/nested/hello.txt")
        .is_file());
}

#[test]
fn run_with_an_empty_destination_keeps_the_dialog_and_says_why() {
    let temp = TempDir::new().expect("temp");
    pack(temp.path(), "a.tar.gz");
    let mut app = app_between(temp.path(), temp.path());
    select(&mut app, "a.tar.gz");
    open_extract(&mut app);
    if let Overlay::ExtractPrompt(prompt) = &mut app.overlay {
        prompt.destination.clear();
    }
    run_extract(&mut app);
    assert!(matches!(app.overlay, Overlay::ExtractPrompt(_)));
    assert!(
        app.status_text().contains("destination"),
        "{}",
        app.status_text()
    );
}

#[test]
fn run_reports_the_first_failure_and_keeps_the_marks() {
    let temp = TempDir::new().expect("temp");
    let (left, right) = (temp.path().join("l"), temp.path().join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    pack(&left, "good.tar");
    fs::write(left.join("bad.tar.gz"), "not gzip at all").expect("file");
    let mut app = app_between(&left, &right);
    for name in ["bad.tar.gz", "good.tar"] {
        select(&mut app, name);
        app.get_active_panel().toggle_mark_selected();
    }
    open_extract(&mut app);
    run_extract(&mut app);

    let status = app.status_text();
    assert!(status.starts_with("Extracted 1 of 2 archives"), "{status}");
    assert!(status.contains("bad.tar.gz"), "{status}");
    assert_eq!(
        app.get_active_panel().marked.len(),
        2,
        "a failure keeps the selection"
    );
    assert!(right.join("payload/nested/hello.txt").is_file());
}

#[test]
fn run_with_nothing_to_extract_closes_and_reports() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("notes.txt"), "x").expect("file");
    let mut app = app_between(temp.path(), temp.path());
    select(&mut app, "notes.txt");
    open_extract(&mut app);
    if let Overlay::ExtractPrompt(prompt) = &mut app.overlay {
        prompt.field = ExtractField::Destination;
    }
    run_extract(&mut app);
    assert!(!app.overlay.is_active());
    assert!(
        app.status_text().contains("No archive selected"),
        "{}",
        app.status_text()
    );
}
