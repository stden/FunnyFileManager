//! Purpose: Unit tests for AI assistant interactive action guards, busy state, and network init.

use std::sync::Arc;

use super::{
    clear_history, context_for, cycle_provider, initial_dialog, insert_command, publish,
    save_transcript, scroll_history, select_provider, submit_prompt,
};
use crate::models::services::ai::{AiDialog, AiRequestId};
use crate::models::{App, Overlay, Panel};
use crate::test_support::{EnvVarGuard, CONFIG_HOME};
use crate::values::ai::{AiAccess, AiError, AiProvider, AiSettings};

fn app_in(dir: &std::path::Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

#[test]
fn context_for_captures_selected_and_marked_entries() {
    let temp = tempfile::TempDir::new().expect("temp");
    std::fs::write(temp.path().join("notes.txt"), "hello").expect("file");
    let mut app = app_in(temp.path());
    let panel = app.get_active_panel();
    assert!(panel.select_named("notes.txt"));
    assert!(panel.toggle_mark_selected());
    let ctx = context_for(&app);
    assert_eq!(ctx.selected.as_deref(), Some("notes.txt"));
    assert_eq!(ctx.marked, vec!["notes.txt".to_owned()]);
}

#[test]
fn actions_no_op_when_ai_overlay_is_not_active() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    submit_prompt(&mut app);
    cycle_provider(&mut app);
    insert_command(&mut app);
    save_transcript(&mut app).expect("save");
    clear_history(&mut app);
    scroll_history(&mut app, -1);
    select_provider(&mut app, AiProvider::OpenAi);
    publish(
        &mut app,
        &Arc::new(()),
        AiRequestId::default(),
        Err(AiError::Worker),
    );
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn busy_dialog_blocks_prompt_and_provider_changes_and_publish_ignores_stale() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    let mut dialog = AiDialog::new(AiSettings::default());
    dialog.prompt.set_text("queued");
    assert!(dialog.queue_prompt(context_for(&app)));
    app.set_overlay(Overlay::Ai(Box::new(dialog)));

    submit_prompt(&mut app);
    cycle_provider(&mut app);
    select_provider(&mut app, AiProvider::Codex);
    assert!(matches!(&app.overlay, Overlay::Ai(d) if d.provider == AiProvider::Anthropic));

    publish(
        &mut app,
        &Arc::new(()),
        AiRequestId::default(),
        Err(AiError::Worker),
    );
    assert!(matches!(&app.overlay, Overlay::Ai(d) if d.is_busy()));
}

#[test]
fn initial_dialog_reads_default_settings_when_network_enabled() {
    let temp = tempfile::TempDir::new().expect("temp");
    let dir = temp.path().join("ffm");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join(crate::config::document::FILE_NAME),
        "ai:\n  provider: openai\n",
    )
    .expect("settings");
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let mut app = app_in(temp.path());
    app.live.ai = AiAccess::Network;
    let dialog = initial_dialog(&app, None, None);
    assert_eq!(dialog.provider, AiProvider::OpenAi);
}
