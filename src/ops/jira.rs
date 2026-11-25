//! Purpose: Command handlers and background coordination helpers for the Jira browser dialog.

pub mod actions;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use crate::command::JiraCommand;
use crate::models::services::jira::JiraDialog;
use crate::models::{App, Overlay};
use crate::status::Status;
use crate::values::jira::{
    JiraAccess, JiraError, JiraReply, JiraRequest, JiraSettings, DEFAULT_JQL, DEFAULT_SNAPSHOT_JQL,
};

/// Open the Jira issues dialog using `ffm.yml` + environment configuration.
pub fn open(app: &mut App) {
    open_with_jql(app, None);
}

/// Open the Jira issues dialog with an optional custom JQL filter.
pub fn open_with_jql(app: &mut App, jql_override: Option<&str>) {
    let settings = active_settings(app);
    let export_dir = resolve_export_dir(app, &settings);
    let jql = effective_jql(&settings, jql_override);
    let dialog = match crate::jira::config::resolve_config(&settings) {
        Ok(cfg) => JiraDialog::new_ready(jql, cfg.page_size, export_dir, cfg.base_url),
        Err(_) if app.live.jira == JiraAccess::Offline => {
            JiraDialog::new_ready(jql, 50, export_dir, "https://jira.example.test".to_owned())
        }
        Err(error) => JiraDialog::new_setup(error.to_string(), jql, export_dir),
    };
    app.set_overlay(Overlay::Jira(Box::new(dialog)));
}

/// Dispatch `:jira ...` CLI commands into dialog actions or config editing.
pub fn command(app: &mut App, cmd: JiraCommand) -> std::io::Result<()> {
    match cmd {
        JiraCommand::Browse | JiraCommand::Export(None) => {
            open(app);
            Ok(())
        }
        JiraCommand::Jql(jql) => {
            open_with_jql(app, Some(&jql));
            Ok(())
        }
        JiraCommand::Export(Some(key)) => {
            open_export_key(app, key);
            Ok(())
        }
        JiraCommand::Snapshot(jql) => {
            open_snapshot(app, jql.as_deref());
            Ok(())
        }
        JiraCommand::Edit => edit_config(app),
    }
}

/// Open `ffm.yml` in the built-in editor so `jira:` settings can be edited.
pub fn edit_config(app: &mut App) -> std::io::Result<()> {
    crate::ops::network_drives::edit_config(app)
}

fn open_export_key(app: &mut App, key: String) {
    let settings = active_settings(app);
    let export_dir = resolve_export_dir(app, &settings);
    let jql = effective_jql(&settings, None);
    let mut dialog = match prepare_dialog(app, &settings, jql, &export_dir) {
        Ok((dialog, _)) => dialog,
        Err(err_dialog) => return app.set_overlay(Overlay::Jira(err_dialog)),
    };
    dialog.queue(JiraRequest::ExportKey {
        key,
        output_dir: export_dir,
        force: false,
    });
    app.set_overlay(Overlay::Jira(dialog));
}

fn open_snapshot(app: &mut App, jql_override: Option<&str>) {
    let settings = active_settings(app);
    let export_dir = resolve_export_dir(app, &settings);
    let jql = jql_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SNAPSHOT_JQL)
        .to_owned();
    let (mut dialog, page_size) = match prepare_dialog(app, &settings, jql.clone(), &export_dir) {
        Ok(pair) => pair,
        Err(err_dialog) => return app.set_overlay(Overlay::Jira(err_dialog)),
    };
    dialog.queue(JiraRequest::ExportSnapshot {
        jql,
        page_size,
        output_dir: export_dir,
    });
    app.set_overlay(Overlay::Jira(dialog));
}

fn boxed_setup(status: String, jql: String, export_dir: &std::path::Path) -> Box<JiraDialog> {
    Box::new(JiraDialog::new_setup(status, jql, export_dir.to_path_buf()))
}

fn prepare_dialog(
    app: &App,
    settings: &JiraSettings,
    jql: String,
    export_dir: &std::path::Path,
) -> Result<(Box<JiraDialog>, u16), Box<JiraDialog>> {
    match crate::jira::config::resolve_config(settings) {
        Ok(cfg) => Ok((boxed_setup(String::new(), jql, export_dir), cfg.page_size)),
        Err(_) if app.live.jira == JiraAccess::Offline => {
            Ok((boxed_setup(String::new(), jql, export_dir), 50))
        }
        Err(error) => Err(boxed_setup(error.to_string(), jql, export_dir)),
    }
}

fn active_settings(app: &App) -> JiraSettings {
    if app.live.jira == JiraAccess::Network {
        crate::jira::config::load_default_settings()
    } else {
        JiraSettings::default()
    }
}

fn resolve_export_dir(app: &App, settings: &JiraSettings) -> PathBuf {
    settings
        .export_dir
        .clone()
        .unwrap_or_else(|| app.get_active_panel_ref().path.clone())
}

fn effective_jql(settings: &JiraSettings, jql_override: Option<&str>) -> String {
    jql_override
        .and_then(|s| (!s.trim().is_empty()).then(|| s.trim().to_owned()))
        .or_else(|| {
            settings
                .jql
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| DEFAULT_JQL.to_owned())
}

/// Apply a finished Jira background reply if the same dialog is still waiting on `id`.
pub fn publish(
    app: &mut App,
    generation: &Arc<()>,
    id: crate::models::services::jira::JiraRequestId,
    reply: JiraReply,
) {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) || !dialog.awaits(id) {
        return;
    }
    match dialog.apply_reply(reply) {
        Ok(Some(report)) => {
            app.console.record_output(&report.summary);
            drop(crate::utils::refresh_panels(app));
            app.set_status(Status::CommandOutput(report.summary));
        }
        Ok(None) => {
            let msg = dialog.status.clone();
            app.set_status(Status::CommandOutput(msg));
        }
        Err(error) => {
            let msg = format_err(&error);
            dialog.status.clone_from(&msg);
            app.set_status(Status::Error(msg));
        }
    }
}

fn format_err(error: &JiraError) -> String {
    error.to_string()
}
