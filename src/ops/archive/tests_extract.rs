//! Unit tests: Shift+F2 unpacks every selected archive into the other panel,
//! says what it did in the status bar and the console, and refreshes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use super::{extract_archive, is_archive, open_extract, run_extract};
use crate::models::{App, ExtractField, Overlay, Panel, PanelSide};
use crate::status::Status;

/// A folder holding one recognisable file, packed as `name` next to it.
fn pack(root: &Path, name: &str) -> PathBuf {
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("nested")).expect("payload dir");
    fs::write(payload.join("nested/hello.txt"), "hello").expect("payload file");
    let archive = root.join(name);
    let is_zip = Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    let status = if is_zip {
        Command::new("python3")
            .args(["-m", "zipfile", "-c"])
            .arg(&archive)
            .arg("payload")
            .current_dir(root)
            .status()
    } else {
        Command::new("tar")
            .arg("-caf")
            .arg(&archive)
            .arg("payload")
            .current_dir(root)
            .status()
    };
    assert!(status.expect("packer runs").success(), "packing {name}");
    fs::remove_dir_all(&payload).expect("payload removed");
    archive
}

fn has(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Two panels: `left` (active, holding the archives) and `right` (destination).
fn app_between(left: &Path, right: &Path) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(left.to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(right.to_path_buf()).expect("right panel");
    app.active_panel = PanelSide::Left;
    app
}

fn select(app: &mut App, name: &str) {
    let panel = app.get_active_panel();
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("entry listed");
    panel.select(Some(index));
}

fn console_text(app: &App) -> String {
    app.console
        .visible(1000)
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn status_text(app: &App) -> String {
    match &app.status {
        Status::CommandOutput(text) => text.clone(),
        other => format!("{other:?}"),
    }
}

#[test]
fn every_far_extension_is_an_archive_and_nothing_else_is() {
    for name in [
        "a.tar",
        "a.tar.gz",
        "a.tgz",
        "a.tar.xz",
        "a.tar.bz2",
        "a.zip",
        "A.ZIP",
    ] {
        assert!(is_archive(name), "{name}");
    }
    for name in [
        "a.txt",
        "tar",
        "a.gz",
        "a.rar",
        "a.7z",
        ".zip",
        "archive.zip.txt",
    ] {
        assert!(!is_archive(name), "{name}");
    }
}

#[test]
fn extract_archive_unpacks_each_tar_flavour() {
    let temp = TempDir::new().expect("temp");
    for name in ["a.tar", "b.tar.gz", "c.tgz", "d.tar.xz", "e.tar.bz2"] {
        let archive = pack(temp.path(), name);
        let destination = temp.path().join(format!("out-{name}"));
        extract_archive(&archive, &destination).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            fs::read_to_string(destination.join("payload/nested/hello.txt")).expect("unpacked"),
            "hello",
            "{name}"
        );
    }
}

#[test]
fn extract_archive_unpacks_a_zip_with_unzip() {
    if !has("unzip") || !has("python3") {
        return;
    }
    let temp = TempDir::new().expect("temp");
    let archive = pack(temp.path(), "a.zip");
    let destination = temp.path().join("out");
    extract_archive(&archive, &destination).expect("unzip");
    assert!(destination.join("payload/nested/hello.txt").is_file());
}

/// A tar holding a symlink is refused before anything is written — the
/// destination stays empty.
#[cfg(unix)]
#[test]
fn extract_archive_refuses_a_tar_with_a_link() {
    let temp = TempDir::new().expect("temp");
    let payload = temp.path().join("payload");
    fs::create_dir_all(&payload).expect("payload");
    fs::write(payload.join("hello.txt"), "hello").expect("file");
    std::os::unix::fs::symlink("/etc/hostname", payload.join("link")).expect("symlink");
    let archive = temp.path().join("linked.tar");
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("payload")
        .current_dir(temp.path())
        .status()
        .expect("tar runs");
    assert!(status.success());
    let destination = temp.path().join("out");
    let error = extract_archive(&archive, &destination).expect_err("a link is refused");
    assert!(error.to_string().contains("linked.tar"), "{error}");
    assert!(!destination.exists(), "nothing was written");
}

/// A zip with a `..` member is refused before `unzip` runs.
#[test]
fn extract_archive_refuses_a_zip_with_a_parent_path() {
    if !has("unzip") || !has("python3") {
        return;
    }
    let temp = TempDir::new().expect("temp");
    let escaping = temp.path().join("escape.zip");
    let script = format!(
        "import zipfile\nwith zipfile.ZipFile({escaping:?}, 'w') as z:\n    z.writestr('../escape.txt', 'x')\n"
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg(script)
        .status()
        .expect("python3 runs");
    assert!(status.success());
    let destination = temp.path().join("out");
    let error = extract_archive(&escaping, &destination).expect_err("a parent path is refused");
    assert!(error.to_string().contains("outside"), "{error}");
    assert!(!destination.exists(), "nothing was written");
}

#[test]
fn extract_archive_refuses_what_it_cannot_unpack() {
    let temp = TempDir::new().expect("temp");
    let bogus = temp.path().join("notes.txt");
    fs::write(&bogus, "text").expect("file");
    let error = extract_archive(&bogus, temp.path()).expect_err("not an archive");
    assert!(error.to_string().contains("notes.txt"), "{error}");

    let broken = temp.path().join("broken.tar.gz");
    fs::write(&broken, "this is not gzip").expect("file");
    assert!(extract_archive(&broken, &temp.path().join("out")).is_err());
}

#[test]
fn open_offers_the_other_panel_and_the_archive_under_the_cursor() {
    let temp = TempDir::new().expect("temp");
    let (left, right) = (temp.path().join("l"), temp.path().join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    let archive = pack(&left, "a.tar.gz");
    let mut app = app_between(&left, &right);
    select(&mut app, "a.tar.gz");

    open_extract(&mut app);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("dialog opens");
    };
    assert_eq!(prompt.archives, vec![archive]);
    assert_eq!(prompt.destination, right.to_string_lossy());
    assert!(prompt.can_extract());
}

#[test]
fn open_on_a_plain_file_says_so_and_offers_no_extract() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("notes.txt"), "x").expect("file");
    let mut app = app_between(temp.path(), temp.path());
    select(&mut app, "notes.txt");
    open_extract(&mut app);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("dialog opens");
    };
    assert!(prompt.archives.is_empty());
    assert!(!prompt.can_extract());
}

/// `l/` holding two archives and a note, all marked; `r/` empty and inactive.
fn two_marked_archives(temp: &Path) -> (App, PathBuf) {
    let (left, right) = (temp.join("l"), temp.join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    pack(&left, "a.tar.gz");
    pack(&left, "b.tar");
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
    let status = status_text(&app);
    assert!(status.starts_with("Extracted 2 archives to "), "{status}");
    assert!(app.get_active_panel().marked.is_empty(), "marks are spent");
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
        prompt.destination = "unpacked/here".to_owned();
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
        status_text(&app).contains("destination"),
        "{}",
        status_text(&app)
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

    let status = status_text(&app);
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
        status_text(&app).contains("No archive selected"),
        "{}",
        status_text(&app)
    );
}
