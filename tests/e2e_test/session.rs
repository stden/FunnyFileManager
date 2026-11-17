//! Purpose: The opening screen, panel switching, the command line and the information panel.

use super::*;

// ─── the panels ─────────────────────────────────────────────────────────────

/// The starting screen: a listing with Far's four columns and the key bar.
#[test]
fn e2e_a_session_opens_on_two_panels_and_a_key_bar() {
    let (left, right) = fixture(&[("alpha.txt", "a")]);
    let screen = run(left.path(), right.path(), quit());

    assert!(screen.contains("alpha"), "the file is listed:\n{screen}");
    assert!(screen.contains("Name"), "column headers:\n{screen}");
    assert!(screen.contains("Size"), "column headers:\n{screen}");
    assert!(screen.contains("Date"), "column headers:\n{screen}");
    assert!(screen.contains("Time"), "column headers:\n{screen}");
    assert!(screen.contains("1Help"), "the key bar:\n{screen}");
    assert!(screen.contains("12Screen"), "all twelve keys:\n{screen}");
}

/// Far names a directory rather than measuring it, and prints a file's byte
/// count outright.
#[test]
fn e2e_the_size_column_names_folders_and_counts_files() {
    let left = TempDir::new().expect("dir");
    let right = TempDir::new().expect("dir");
    fs::create_dir(left.path().join("docs")).expect("dir");
    fs::write(left.path().join("data.bin"), vec![0u8; 4096]).expect("file");

    let screen = run(left.path(), right.path(), quit());

    assert!(screen.contains("Folder"), "directories:\n{screen}");
    assert!(screen.contains("Up"), "the parent row:\n{screen}");
    assert!(screen.contains("4096"), "the exact byte count:\n{screen}");
}

/// Far splits a file row at its final dot and right-aligns the extension.
#[test]
fn e2e_a_file_name_is_split_from_its_extension() {
    let (left, right) = fixture(&[("report.md", "x")]);
    let screen = run(left.path(), right.path(), quit());

    let row = screen
        .lines()
        .find(|line| line.contains("report"))
        .expect("the file is listed");
    assert!(row.contains("report"), "{row}");
    assert!(row.contains("md"), "the extension is shown: {row}");
    assert!(
        !row.contains("report.md"),
        "name and extension are separate columns: {row}"
    );
}

/// Tab moves the cursor bar to the other panel, which is what makes every
/// two-panel operation aim somewhere.
#[test]
fn e2e_tab_switches_the_active_panel() {
    let (left, right) = fixture(&[("only-left.txt", "x")]);
    fs::write(right.path().join("only-right.txt"), "y").expect("seed");

    let mut events = vec![key(KeyCode::Tab)];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(screen.contains("only-right"), "both panels draw:\n{screen}");
}

// ─── the command line ───────────────────────────────────────────────────────

/// A printable key opens the command line holding it, the way a shell does.
#[test]
fn e2e_typing_opens_the_command_line_and_a_command_runs() {
    let (left, right) = fixture(&[]);

    let mut events = typed("mkdir made-by-command");
    events.push(key(KeyCode::Enter));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("made-by-command").is_dir(),
        "the typed command must run"
    );
}

/// Esc abandons a typed command rather than running it.
#[test]
fn e2e_escape_abandons_a_typed_command() {
    let (left, right) = fixture(&[]);

    let mut events = typed("mkdir never-made");
    events.push(key(KeyCode::Esc));
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        !left.path().join("never-made").exists(),
        "an abandoned command must not run"
    );
}

// ─── directory statistics ───────────────────────────────────────────────────

/// Ctrl+L measures the active directory and says where the space is.
#[test]
fn e2e_ctrl_l_reports_what_the_directory_contains() {
    let left = TempDir::new().expect("dir");
    let right = TempDir::new().expect("dir");
    fs::create_dir_all(left.path().join("heavy")).expect("dir");
    fs::write(left.path().join("heavy/blob.bin"), vec![0u8; 50_000]).expect("file");
    fs::write(left.path().join("main.rs"), "fn main() {}").expect("file");
    fs::write(left.path().join("notes.md"), "hello").expect("file");

    let screen = run_until(
        left.path(),
        right.path(),
        vec![key_with(KeyCode::Char('l'), KeyModifiers::CONTROL)],
    );

    assert!(screen.contains("Information"), "the panel opens:\n{screen}");
    assert!(screen.contains("Largest folders"), "{screen}");
    assert!(
        screen.contains("heavy"),
        "the big folder is named:\n{screen}"
    );
    assert!(screen.contains("By content"), "{screen}");
    assert!(
        screen.contains("Source"),
        "the .rs file is categorised:\n{screen}"
    );
    assert!(screen.contains("Documents"), "and the .md file:\n{screen}");
    assert!(
        screen.contains("3 files"),
        "the totals are there:\n{screen}"
    );
}

/// …and Esc puts it away again.
#[test]
fn e2e_the_information_panel_closes() {
    let (left, right) = fixture(&[("a.txt", "x")]);

    let mut events = vec![
        key_with(KeyCode::Char('l'), KeyModifiers::CONTROL),
        key(KeyCode::Esc),
    ];
    events.extend(quit());
    let screen = run(left.path(), right.path(), events);

    assert!(
        !screen.contains("Largest folders"),
        "the panel must close:\n{screen}"
    );
}
