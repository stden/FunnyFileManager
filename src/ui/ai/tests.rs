//! Purpose: Rendering tests for the AI assistant overlay across providers and transcripts.

use crate::models::services::ai::AiDialog;
use crate::models::{App, Overlay, Panel};
use crate::ui::overlays::render_overlay_text as render_text;
use crate::values::ai::{AiProvider, AiResponse, AiSettings};

#[test]
fn renders_ai_empty_and_transcript_states() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let mut dialog = AiDialog::new(AiSettings::default());
    app.set_overlay(Overlay::Ai(Box::new(dialog.clone())));
    assert!(render_text(&mut app).contains("[Anthropic]"));

    dialog.set_provider(AiProvider::Codex);
    dialog.apply_reply(Ok(AiResponse {
        provider: AiProvider::Codex,
        model: "gpt-5.3-codex".into(),
        text: "Use this:\n\n```sh\ngit status\n```".into(),
        command: Some("git status".into()),
        input_tokens: Some(3),
        output_tokens: Some(5),
    }));
    app.set_overlay(Overlay::Ai(Box::new(dialog)));
    let screen = render_text(&mut app);
    assert!(screen.contains("[Codex]") && screen.contains("git status"));
}

#[test]
fn renders_ai_safely_when_terminal_area_is_tiny() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let dialog = AiDialog::new(AiSettings::default());
    app.set_overlay(Overlay::Ai(Box::new(dialog)));
    let backend = ratatui::backend::TestBackend::new(4, 4);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .expect("render");
}
