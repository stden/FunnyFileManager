//! Purpose: Key routing tests for the Warp-like AI assistant chat overlay.

use super::{app_in, press};
use crate::models::services::ai::AiDialog;
use crate::models::Overlay;
use crate::values::ai::{AiProvider, AiResponse, AiSettings};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn ai_prompt_provider_switch_insert_save_clear_and_scroll() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let mut dialog = AiDialog::new(AiSettings::default());
    dialog.apply_reply(Ok(AiResponse {
        provider: AiProvider::Anthropic,
        model: "claude-haiku-5-5".into(),
        text: "Run:\n```sh\nls -la\n```".into(),
        command: Some("ls -la".into()),
        input_tokens: Some(4),
        output_tokens: Some(8),
    }));
    app.set_overlay(Overlay::Ai(Box::new(dialog)));

    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('1'), KeyModifiers::ALT);
    press(&mut app, KeyCode::Char('2'), KeyModifiers::ALT);
    press(&mut app, KeyCode::Char('3'), KeyModifiers::ALT);
    press(&mut app, KeyCode::Up, KeyModifiers::NONE);
    press(&mut app, KeyCode::PageUp, KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    press(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut app, KeyCode::F(2), KeyModifiers::NONE);
    assert!(temp.path().join("ai-transcript.md").exists());
    press(&mut app, KeyCode::F(5), KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}

#[test]
fn ai_clear_ctrl_x_editor_and_escape_keys() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let mut dialog = AiDialog::new(AiSettings::default());
    dialog.last_command = Some("pwd".into());
    app.set_overlay(Overlay::Ai(Box::new(dialog)));
    press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(matches!(&app.overlay, Overlay::CommandLine(cl) if cl.text() == "pwd"));

    let mut to_clear = AiDialog::new(AiSettings::default());
    to_clear.last_command = Some("pwd".into());
    app.set_overlay(Overlay::Ai(Box::new(to_clear)));
    press(&mut app, KeyCode::F(8), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert!(matches!(&app.overlay, Overlay::Ai(d) if d.last_command.is_none()));

    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    press(&mut app, KeyCode::F(4), KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::Editor(_)));
    app.set_overlay(Overlay::Ai(Box::new(AiDialog::new(AiSettings::default()))));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(matches!(app.overlay, Overlay::None));
    assert!(!super::process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
    super::super::ai::process(&mut app, KeyCode::Char('a'), KeyModifiers::NONE);
}
