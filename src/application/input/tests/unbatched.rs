//! Purpose: where typing is not batched into a paste — outside the editor's document, behind its dialogs, and in a replay.

use super::*;

#[test]
fn batching_is_disabled_outside_the_editor_document() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.get_editor_mut().unwrap().state = EditState::ConfirmingExit(ExitChoice::Save);
    assert_unbatched(&app)?;
    app.get_editor_mut().unwrap().state = EditState::Clean;
    app.get_editor_mut().unwrap().search.prompt = Some(crate::search::SearchPrompt::default());
    assert_unbatched(&app)?;
    app.get_editor_mut().unwrap().search.prompt = None;
    app.get_editor_mut().unwrap().encoding_choice = Some(crate::encoding::TextEncoding::Utf8);
    assert_unbatched(&app)?;
    app.close_overlay();
    assert_unbatched(&app)
}

fn assert_unbatched(app: &App) -> io::Result<()> {
    let mut input = queued(text_events("ab"));
    assert_eq!(input.read(app)?, Some(key(KeyCode::Char('a'))));
    assert_eq!(input.read(app)?, Some(key(KeyCode::Char('b'))));
    Ok(())
}

#[test]
fn terminal_paste_cannot_edit_the_document_behind_a_subdialog() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut app = app_at(temp.path());
    app.get_editor_mut().unwrap().search.prompt = Some(crate::search::SearchPrompt::default());
    crate::popup::paste_text(&mut app, "search text");
    app.get_editor_mut().unwrap().search.prompt = None;
    app.get_editor_mut().unwrap().encoding_choice = Some(crate::encoding::TextEncoding::Utf8);
    crate::popup::paste_text(&mut app, "encoding text");
    app.get_editor_mut().unwrap().encoding_choice = None;
    app.get_editor_mut().unwrap().state = EditState::ConfirmingExit(ExitChoice::Save);
    crate::popup::paste_text(&mut app, "confirmation text");
    assert_eq!(app.get_editor().unwrap().content(), "old");
    app.close_overlay();
    crate::popup::paste_text(&mut app, "panels");
    assert!(matches!(app.overlay, Overlay::None));
    Ok(())
}

/// A replayed session keeps its recorded key boundaries: typing that was
/// recorded key by key must not come back as one paste, even when every key
/// is already due.
#[test]
fn replayed_typing_is_delivered_key_by_key_not_batched_into_a_paste() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let recording = temp.path().join("events.jsonl");
    let header = r#"{"version":1,"executable":"ffm","directory":".","size":[80,24]}"#;
    let rows: Vec<String> = text_events("ab")
        .map(|event| serde_json::json!({ "delay_ms": 0, "event": event }).to_string())
        .collect();
    std::fs::write(&recording, format!("{header}\n{}\n", rows.join("\n")))?;
    let (mut replay, _) = crate::recording::Input::from_args(
        &["--replay".into(), recording.into_os_string()],
        (80, 24),
    )?;
    let mut input = BufferedInput::new(&mut replay);

    assert_eq!(input.read(&app)?, Some(key(KeyCode::Char('a'))));
    assert_eq!(input.read(&app)?, Some(key(KeyCode::Char('b'))));
    assert_eq!(
        input.read(&app).expect_err("end of recording").kind(),
        io::ErrorKind::UnexpectedEof
    );
    Ok(())
}
