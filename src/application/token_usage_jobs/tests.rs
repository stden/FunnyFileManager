//! Purpose: Actual worker publication, report rendering and late reply protection.
use super::*;
use crate::models::token_usage::{Tokens, UsageTotals};
use crate::models::Panel;
fn app(temp: &tempfile::TempDir) -> App {
    let panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    App::with_panels(panel.clone(), panel)
}
fn fixture() -> Reports {
    vec![ProviderUsage {
        provider: Provider::OpenAi,
        result: Ok(UsageTotals {
            input: Tokens::new(100),
            output: Tokens::new(20),
            cached: Some(Tokens::new(40)),
            requests: 2,
        }),
    }]
}
#[test]
fn usage_menu_opens_and_background_results_are_published() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::menu::execute(&mut app, crate::menu::MenuCommand::TokenUsage);
    let mut jobs = UsageJobs::default();
    jobs.pump_with(&mut app, fixture);
    finish(&mut jobs, &mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("usage panel");
    };
    assert_eq!(panel.state, UsageState::Ready);
    assert!(panel
        .viewer
        .lines
        .iter()
        .any(|line| line.contains("OpenAI") && line.contains("100")));
}
#[test]
fn late_report_cannot_reopen_a_closed_panel() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let mut jobs = UsageJobs::default();
    jobs.pump_with(&mut app, fixture);
    app.close_overlay();
    finish(&mut jobs, &mut app);
    assert!(matches!(app.overlay, Overlay::None));
}
fn finish(jobs: &mut UsageJobs, app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while jobs.pending.is_some() {
        jobs.pump_with(app, fixture);
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn usage_panel_renders_errors_and_keyboard_refresh_is_modal() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let Overlay::TokenUsage(panel) = &mut app.overlay else {
        panic!("panel");
    };
    crate::ops::token_usage::publish(panel, &failed());
    assert!(panel
        .viewer
        .lines
        .iter()
        .any(|line| line.contains(Provider::OpenAi.key_variable())));
    assert_eq!(Provider::Anthropic.label(), "Anthropic");
    let text = render_text(&mut app);
    assert!(text.contains("OpenRouter"));
    assert!(crate::popup::process_popup_input(
        &mut app,
        crossterm::event::KeyCode::F(5).into()
    ));
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert_eq!(panel.state, UsageState::Queued);
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::Esc.into());
    assert!(matches!(app.overlay, Overlay::None));
}

fn render_text(app: &mut App) -> String {
    let backend = ratatui::backend::TestBackend::new(120, 24);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| crate::ui::ui(frame, app))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[test]
fn isolated_sessions_do_not_read_provider_keys_or_call_external_apis() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let mut jobs = UsageJobs::default();
    jobs.pump(&mut app);
    finish(&mut jobs, &mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert!(panel
        .viewer
        .lines
        .iter()
        .any(|line| line.contains(UsageError::Disabled.message())));
}
