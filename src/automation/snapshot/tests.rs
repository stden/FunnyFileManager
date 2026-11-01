//! Purpose: Passive state exposes editor coordinates while redacting revealed vault secrets.
use super::*;
use crate::{automation::options::Profile, models::Editor};
use ratatui::layout::Size;

#[test]
fn editor_snapshot_exposes_metadata_without_its_document_content() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let mut editor = Editor::from_content(
        "notes.txt".into(),
        directory.path().join("notes.txt"),
        "private document\nsecond line",
    );
    editor.cursor_line = 1;
    editor.cursor_col = 4;
    runtime.app.set_overlay(Overlay::Editor(editor));
    let snapshot = capture(&runtime);
    assert_eq!(snapshot["editor"]["cursor_line"], 1);
    assert_eq!(snapshot["editor"]["cursor_column"], 4);
    assert_eq!(snapshot["editor"]["line_count"], 2);
    assert_eq!(snapshot["editor"]["modified"], false);
    assert_eq!(snapshot["modal"]["kind"], "editor");
    assert!(!snapshot["editor"].to_string().contains("private document"));
}

#[cfg(feature = "keepass")]
#[test]
fn revealed_unlock_prompt_redacts_all_passive_screen_rows() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let mut session = crate::kdbx::KdbxSession::new(directory.path().join("vault.kdbx"));
    crate::kdbx::tests_support::type_password(
        &mut session,
        "fixture-passphrase",
        std::time::Instant::now(),
    );
    session.prompt_mut().unwrap().toggle_reveal();
    runtime
        .app
        .set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    assert!(painted_text(&mut runtime).contains("fixture-passphrase"));
    let snapshot = capture(&runtime);
    assert_eq!(snapshot["screen"]["redacted"], true);
    assert_eq!(
        snapshot["screen"]["rows"],
        json!(["[Revealed secret omitted from passive screen state]"])
    );
    assert!(snapshot["editor"].is_null());
    assert!(!snapshot.to_string().contains("fixture-passphrase"));
}

#[cfg(feature = "keepass")]
#[test]
fn unlocked_vault_is_redacted_only_while_secret_is_revealed() {
    let directory = tempfile::TempDir::new().unwrap();
    let mut runtime = Runtime::new(directory.path(), Size::new(80, 24), Profile::Isolated).unwrap();
    let (_vault_directory, mut session, now) = crate::kdbx::tests_support::unlocked_sample();
    crate::kdbx::tests_support::focus(&mut session, "GitHub", now);
    assert!(!session.vault().unwrap().is_revealed());
    session.reveal(now).unwrap();
    runtime
        .app
        .set_overlay(Overlay::Kdbx(crate::models::KdbxOverlay(session)));
    assert!(painted_text(&mut runtime).contains("gh-pw"));
    let snapshot = capture(&runtime);
    assert_eq!(snapshot["screen"]["redacted"], true);
    assert!(!snapshot.to_string().contains("gh-pw"));
}

#[cfg(feature = "keepass")]
fn painted_text(runtime: &mut Runtime) -> String {
    runtime
        .terminal
        .draw(|frame| crate::ui::ui(frame, &mut runtime.app))
        .unwrap();
    runtime
        .terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}
