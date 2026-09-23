//! Queued terminal text is one insertion, with key boundaries and errors retained.

use std::{collections::VecDeque, path::Path};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::*;
use crate::models::{EditState, Editor, ExitChoice, Overlay, Panel};

struct Queue(VecDeque<io::Result<Event>>);

impl EventSource for Queue {
    fn read(&mut self) -> io::Result<Option<Event>> {
        self.0.pop_front().transpose()
    }

    fn try_read(&mut self) -> io::Result<Option<Event>> {
        self.read()
    }
}

fn app_at(path: &Path) -> App {
    let mut app = App::with_panels(
        Panel::from_entries(path.to_owned(), Vec::new()),
        Panel::from_entries(path.to_owned(), Vec::new()),
    );
    app.set_overlay(Overlay::Editor(Editor::from_content(
        "text".to_owned(),
        path.join("text.txt"),
        "old",
    )));
    app
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn queued(events: impl IntoIterator<Item = Event>) -> BufferedInput<Queue> {
    BufferedInput::new(Queue(events.into_iter().map(Ok).collect()))
}

fn text_events(text: &str) -> impl Iterator<Item = Event> + '_ {
    text.chars().map(|ch| {
        key(if ch == '\n' {
            KeyCode::Enter
        } else {
            KeyCode::Char(ch)
        })
    })
}

#[test]
fn queued_unicode_and_newlines_become_a_single_paste() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let mut input = queued(text_events("Hello мир🙂\nsecond line\n"));
    assert_eq!(
        input.read(&app)?,
        Some(Event::Paste("Hello мир🙂\nsecond line\n".to_owned()))
    );
    assert_eq!(input.read(&app)?, None);
    Ok(())
}

#[test]
fn shortcuts_remain_separate_and_in_order() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let copy = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    let mut input = queued(text_events("ab").chain([copy.clone(), key(KeyCode::Esc)]));
    assert_eq!(input.read(&app)?, Some(Event::Paste("ab".to_owned())));
    assert_eq!(input.read(&app)?, Some(copy));
    assert_eq!(input.read(&app)?, Some(key(KeyCode::Esc)));
    Ok(())
}

#[test]
fn windows_release_and_repeat_records_do_not_split_the_text() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let ignored = [KeyEventKind::Release, KeyEventKind::Repeat].map(|kind| {
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            kind,
        ))
    });
    let events = [
        key(KeyCode::Char('界')),
        ignored[0].clone(),
        ignored[1].clone(),
        key(KeyCode::Char('🙂')),
    ];
    let mut input = queued(events);
    assert_eq!(input.read(&app)?, Some(Event::Paste("界🙂".to_owned())));
    Ok(())
}

#[test]
fn single_keys_and_idle_reads_do_not_wait_for_more_input() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let mut reads = 0;
    let mut input = BufferedInput::new(|| {
        reads += 1;
        assert_eq!(reads, 1, "batching must never use the blocking read");
        Ok(Some(key(KeyCode::Char('a'))))
    });
    assert_eq!(input.read(&app)?, Some(key(KeyCode::Char('a'))));
    assert_eq!(queued([]).read(&app)?, None);
    Ok(())
}

#[test]
fn stream_errors_arrive_after_the_text_already_received() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let events = [
        Ok(key(KeyCode::Char('a'))),
        Ok(key(KeyCode::Char('b'))),
        Err(io::Error::other("lost")),
    ];
    let mut input = BufferedInput::new(Queue(events.into()));
    assert_eq!(input.read(&app)?, Some(Event::Paste("ab".to_owned())));
    assert_eq!(input.read(&app).unwrap_err().to_string(), "lost");
    Ok(())
}

#[test]
fn batch_limit_yields_without_losing_the_rest() -> io::Result<()> {
    let temp = TempDir::new()?;
    let app = app_at(temp.path());
    let payload = "界".repeat(MAX_BATCH_EVENTS + 3);
    let mut input = queued(text_events(&payload));
    assert_eq!(
        input.read(&app)?,
        Some(Event::Paste("界".repeat(MAX_BATCH_EVENTS)))
    );
    assert_eq!(input.read(&app)?, Some(Event::Paste("界".repeat(3))));
    Ok(())
}

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
    crate::popup::paste_editor_text(&mut app, "search text");
    app.get_editor_mut().unwrap().search.prompt = None;
    app.get_editor_mut().unwrap().encoding_choice = Some(crate::encoding::TextEncoding::Utf8);
    crate::popup::paste_editor_text(&mut app, "encoding text");
    app.get_editor_mut().unwrap().encoding_choice = None;
    app.get_editor_mut().unwrap().state = EditState::ConfirmingExit(ExitChoice::Save);
    crate::popup::paste_editor_text(&mut app, "confirmation text");
    assert_eq!(app.get_editor().unwrap().content(), "old");
    app.close_overlay();
    crate::popup::paste_editor_text(&mut app, "panels");
    assert!(matches!(app.overlay, Overlay::None));
    Ok(())
}

#[test]
fn zz_probe_recording_input_is_an_event_source() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = temp.path().join("events.jsonl");
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::json!({
                "version": 1,
                "executable": temp.path().join("ffm"),
                "directory": temp.path(),
                "size": [80, 24],
            }),
            serde_json::json!({ "delay_ms": 0, "event": { "Resize": [80, 24] } }),
        ),
    )?;
    let mut recording = crate::recording::Input::from_args(
        &[
            std::ffi::OsString::from("--replay"),
            path.into_os_string(),
        ],
        (80, 24),
    )
    .expect("replay input");
    let app = App::with_panels(
        Panel::from_entries(temp.path().to_owned(), Vec::new()),
        Panel::from_entries(temp.path().to_owned(), Vec::new()),
    );
    let mut buffered = BufferedInput::new(&mut recording);
    assert_eq!(buffered.read(&app)?, Some(Event::Resize(80, 24)));
    Ok(())
}
