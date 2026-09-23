//! Purpose: Inspect configured and cached servers without starting processes or installing packages.

use super::{
    environment::{Environment, System},
    launch::Launch,
    storage, InstallError,
};
use crate::config::{LspSettings, ServerSettings, Settings};
use std::{fs, path::Path};

pub fn inventory(settings: &LspSettings, directory: &Path) -> Vec<String> {
    let mut lines =
        vec!["Configured servers; availability does not mean a server is running.".into()];
    if let Some(path) = Settings::path() {
        lines.push(format!("Settings: {}", path.display()));
    }
    lines.extend(report(
        settings,
        directory,
        super::cache_dir().as_deref(),
        &System,
    ));
    lines
}

fn report(
    settings: &LspSettings,
    directory: &Path,
    cache: Option<&Path>,
    environment: &dyn Environment,
) -> Vec<String> {
    let mut lines =
        vec!["Open a source file with F4 to start its server. Esc closes this report.".into()];
    for (language, server) in &settings.0 {
        lines.push(String::new());
        lines.push(format!(
            "{language} [{}] .{}",
            if server.enabled {
                "enabled"
            } else {
                "disabled"
            },
            server.extensions.join(" .")
        ));
        lines.push(format!("  Configured command: {}", server.command));
        arguments(&mut lines, &server.args, "Configured argument");
        availability(&mut lines, server, directory, cache, environment);
        let recipe = server.install.as_ref().map_or_else(
            || "disabled".into(),
            |recipe| {
                serde_json::to_string(recipe)
                    .expect("installation recipes contain only JSON values")
            },
        );
        lines.push(format!("  Auto-install: {recipe}"));
    }
    lines
}

fn availability(
    lines: &mut Vec<String>,
    server: &ServerSettings,
    directory: &Path,
    cache: Option<&Path>,
    environment: &dyn Environment,
) {
    let workspace =
        super::super::worker::workspace_root(&directory.join("source"), &server.root_markers);
    match resolve(server, cache, &workspace, environment) {
        Ok((state, prepared)) => {
            lines.push(format!("  Status: {state}"));
            lines.push(format!("  Executable: {}", prepared.command));
            arguments(lines, &prepared.args, "Launch argument");
        }
        Err(error) => lines.push(format!("  Status: {}", error.label())),
    }
}

fn arguments(lines: &mut Vec<String>, args: &[String], label: &str) {
    if args.is_empty() {
        lines.push(format!("  {label}s: (none)"));
    }
    for arg in args {
        lines.push(format!("  {label}: {arg}"));
    }
}

fn resolve(
    server: &ServerSettings,
    cache: Option<&Path>,
    workspace: &Path,
    environment: &dyn Environment,
) -> Result<(&'static str, ServerSettings), InstallError> {
    if let Some(path) = configured(&server.command, environment) {
        let mut prepared = server.clone();
        prepared.command = path.to_string_lossy().into_owned();
        return Ok(("found configured executable", prepared));
    }
    let recipe = server.install.as_ref().ok_or(InstallError::Missing(
        "not installed; auto-install disabled",
    ))?;
    let cache = cache.ok_or(InstallError::Missing("no installation cache configured"))?;
    let key = storage::digest(&serde_json::to_vec(recipe).map_err(|_| InstallError::Recipe)?);
    let root = cache.join(key);
    if !root.exists() {
        return Err(InstallError::Missing(
            "not installed; installs after workspace trust",
        ));
    }
    let receipt: Launch = serde_json::from_slice(&fs::read(root.join("ready.json"))?)
        .map_err(|_| InstallError::Recipe)?;
    Ok((
        "installed in FFM cache",
        receipt.preview(server, &root, workspace)?,
    ))
}

fn configured(command: &str, environment: &dyn Environment) -> Option<std::path::PathBuf> {
    let path = Path::new(command);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    if path.components().count() != 1 {
        return None;
    }
    environment.program(&[command], "not installed").ok()
}

#[cfg(test)]
mod tests;
