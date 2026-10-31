//! Purpose: Usage worker failures, generation changes, scrolling, and report details.
use super::*;
use crossterm::event::KeyCode;

#[test]
fn dropped_worker_reports_a_failure_for_each_provider() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    let mut jobs = UsageJobs {
        pending: Some(Pending {
            generation: Arc::clone(&panel.generation),
            receiver,
        }),
        updated: None,
    };
    jobs.poll(&mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert_eq!(panel.state, UsageState::Ready);
    for provider in Provider::ALL {
        assert!(panel
            .viewer
            .lines
            .iter()
            .any(|line| line.contains(provider.label())
                && line.contains(UsageError::Worker.message())));
    }
}

#[test]
fn manual_refresh_discards_the_previous_generation() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    let (sender, receiver) = mpsc::channel();
    sender.send(fixture()).expect("report");
    let mut jobs = UsageJobs {
        pending: Some(Pending {
            generation: Arc::clone(&panel.generation),
            receiver,
        }),
        updated: None,
    };
    crate::popup::process_popup_input(&mut app, KeyCode::F(5).into());
    jobs.poll(&mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert_eq!(panel.state, UsageState::Queued);
    assert!(panel
        .viewer
        .lines
        .iter()
        .all(|line| !line.contains("OpenAI")));
}

#[test]
fn completed_reports_refresh_after_the_interval() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let Overlay::TokenUsage(panel) = &mut app.overlay else {
        panic!("panel");
    };
    crate::ops::token_usage::publish(panel, &[]);
    let mut jobs = UsageJobs {
        updated: Some(
            Instant::now()
                .checked_sub(Duration::from_secs(61))
                .expect("elapsed"),
        ),
        ..UsageJobs::default()
    };
    assert_eq!(jobs.pump_with(&mut app, fixture), Redraw::Needed);
    finish(&mut jobs, &mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert!(panel
        .viewer
        .lines
        .iter()
        .any(|line| line.contains("OpenAI") && line.contains("100")));
}

#[test]
fn published_rows_show_provider_errors() {
    let mut panel = crate::models::TokenUsagePanel::new();
    let mut reports = Vec::new();
    for error in [
        UsageError::MissingKey,
        UsageError::AccessDenied,
        UsageError::RateLimited,
        UsageError::Network,
        UsageError::InvalidResponse,
        UsageError::Limit,
    ] {
        reports.push(ProviderUsage {
            provider: Provider::Anthropic,
            result: Err(error),
        });
    }
    crate::ops::token_usage::publish(&mut panel, &reports);
    for report in &reports {
        let error = report.result.as_ref().expect_err("error");
        assert!(panel
            .viewer
            .lines
            .iter()
            .any(|line| line.contains(error.message())));
    }
}

#[test]
fn published_rows_show_unavailable_cache() {
    let mut panel = crate::models::TokenUsagePanel::new();
    let report = ProviderUsage {
        provider: Provider::OpenRouter,
        result: Ok(UsageTotals::default()),
    };
    crate::ops::token_usage::publish(&mut panel, &[report]);
    assert!(panel
        .viewer
        .lines
        .iter()
        .any(|line| line.contains("OpenRouter") && line.contains("n/a")));
}

#[test]
fn viewer_scroll_is_captured_by_open_usage_panel() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    crate::ops::token_usage::open(&mut app);
    let Overlay::TokenUsage(panel) = &mut app.overlay else {
        panic!("panel");
    };
    panel.viewer.lines = vec!["line".to_owned(); 100];
    assert!(crate::popup::process_popup_input(
        &mut app,
        KeyCode::Down.into()
    ));
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    assert_eq!(panel.viewer.scroll.lines(), 1);
}
