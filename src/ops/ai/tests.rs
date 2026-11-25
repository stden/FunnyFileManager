//! Purpose: Unit tests for AI assistant operations (`open`, `submit_prompt`, `insert_command`, `publish`).

use std::sync::Arc;

use super::actions::{
    clear_history, context_for, cycle_provider, initial_dialog, insert_command, publish,
    save_transcript, scroll_history, select_provider, submit_prompt,
};
use super::*;
use crate::models::Panel;
use crate::values::ai::{AiError, AiResponse};

fn app_in(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn ai_commands_and_context_capture_dual_panels() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let ctx = context_for(&app);
    assert_eq!(ctx.active_dir, temp.path().display().to_string());
    let seeded = initial_dialog(&app, Some("hello"), Some(AiProvider::Codex));
    assert!(seeded.is_busy() && seeded.provider == AiProvider::Codex);

    command(&mut app, AiCommand::Provider(AiProvider::Codex)).expect("codex");
    assert!(matches!(
        &app.overlay,
        Overlay::Ai(d) if d.provider == AiProvider::Codex
    ));
    command(&mut app, AiCommand::Open(Some("find large files".into()))).expect("prompt");
    assert!(matches!(&app.overlay, Overlay::Ai(d) if d.is_busy()));

    let _home =
        crate::test_support::EnvVarGuard::set(crate::test_support::CONFIG_HOME, temp.path());
    command(&mut app, AiCommand::Edit).expect("edit");
    assert!(matches!(app.overlay, Overlay::Editor(_)));
}

#[test]
fn ai_actions_publish_insert_command_and_save_transcript() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    open(&mut app);
    select_provider(&mut app, AiProvider::Anthropic);
    cycle_provider(&mut app);
    if let Overlay::Ai(d) = &mut app.overlay {
        d.prompt.set_text("check disk space");
    }
    submit_prompt(&mut app);
    let (generation, id) = match &mut app.overlay {
        Overlay::Ai(d) => (Arc::clone(&d.generation), d.take_queued().expect("req").0),
        _ => panic!("expected AI overlay"),
    };
    let resp = AiResponse {
        provider: AiProvider::OpenAi,
        model: "gpt-5".into(),
        text: "Run:\n```bash\ndf -h\n```".into(),
        command: Some("df -h".into()),
        input_tokens: Some(12),
        output_tokens: Some(6),
    };
    publish(&mut app, &generation, id, Ok(resp));
    verify_scroll_save_and_insert(&mut app, temp.path(), &generation, id);
}

fn verify_scroll_save_and_insert(
    app: &mut App,
    dir: &std::path::Path,
    generation: &Arc<()>,
    id: crate::models::services::ai::AiRequestId,
) {
    scroll_history(app, -2);
    scroll_history(app, 1);
    save_transcript(app).expect("save md");
    assert!(dir.join("ai-transcript.md").is_file());
    publish(app, generation, id, Err(AiError::Disabled));
    insert_command(app);
    assert!(matches!(&app.overlay, Overlay::CommandLine(cl) if cl.text() == "df -h"));
    open(app);
    insert_command(app);
    clear_history(app);
}
