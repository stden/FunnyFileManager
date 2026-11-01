//! Purpose: Open the usage panel and publish provider reports as readable rows.
use crate::models::token_usage::{ProviderUsage, UsageState};
use crate::models::{App, Overlay, TokenUsagePanel};

pub fn open(app: &mut App) {
    app.set_overlay(Overlay::TokenUsage(Box::new(TokenUsagePanel::new())));
}
pub fn publish(panel: &mut TokenUsagePanel, reports: &[ProviderUsage]) {
    let mut lines = vec![
        "Last 30 completed UTC days | Refreshed automatically every 60 seconds".to_owned(),
        "API account usage; consumer subscriptions are not included.".to_owned(),
        format!(
            "Updated: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ),
        format!(
            "{:<14} {:>16} {:>16} {:>16} {:>12}",
            "Provider", "Input", "Output", "Cached input", "Requests"
        ),
    ];
    for report in reports {
        lines.push(report_line(report));
    }
    lines.extend([
        String::new(),
        "Cached input is part of input, not an additional token charge.".to_owned(),
        "OpenAI: completions usage. Anthropic: organization analytics.".to_owned(),
        "OpenRouter: activity. Provider reporting may be delayed.".to_owned(),
        "F5 refresh | Arrows/PgUp/PgDn scroll | Esc/F10 close".to_owned(),
    ]);
    panel.viewer.lines = lines;
    *panel.viewer.highlights.borrow_mut() = crate::highlight::cache::HighlightCache::default();
    panel.state = UsageState::Ready;
}

fn report_line(report: &ProviderUsage) -> String {
    match &report.result {
        Ok(totals) => format!(
            "{:<14} {:>16} {:>16} {:>16} {:>12}",
            report.provider.label(),
            totals.input.count(),
            totals.output.count(),
            totals
                .cached
                .map_or_else(|| "n/a".to_owned(), |count| count.count().to_string()),
            totals.requests
        ),
        Err(error) => format!(
            "{}: {} ({})",
            report.provider.label(),
            error.message(),
            report.provider.key_variable()
        ),
    }
}
