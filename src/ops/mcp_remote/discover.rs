//! Purpose: Collect MCP JSON configs from Claude Code, Kiro, and `ffm.yml` respecting sandbox isolation.

use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::mcp_clients::{Host, CLIENTS};
use crate::mcp_remote::config;
use crate::ops::mcp_install::survey::{config_path, user_config, Target};
use crate::values::mcp_remote::{HttpMcpServer, McpAccess};

/// Discover Streamable HTTP MCP services configured for the active session.
#[must_use]
pub fn discover_for(
    host: &Host,
    access: McpAccess,
    active_dir: &Path,
    yaml_path: Option<&Path>,
    extra_env: Option<&str>,
    tls_env_override: Option<bool>,
) -> Vec<HttpMcpServer> {
    let mut candidates = Vec::new();
    let folder = crate::mcp_clients::ProjectDir::new(active_dir.to_path_buf());
    for client in &CLIENTS {
        if let Ok(path) = user_config(host, client) {
            candidates.push(path);
        }
        if let Some(dir) = folder.as_ref() {
            candidates.extend(config_path(host, client, Target::CurrentFolder(dir)).ok());
        }
    }
    let resolved_yaml = yaml_path.map(Path::to_path_buf).or_else(|| {
        (access == McpAccess::Network)
            .then(crate::config::dir)
            .flatten()
            .map(|dir| dir.join(crate::config::document::FILE_NAME))
    });
    config::discover(
        &candidates,
        resolved_yaml.as_deref(),
        extra_env,
        tls_env_override,
    )
}

/// Collect live process overrides only when `McpAccess::Network` is enabled.
#[must_use]
pub fn live_overrides(access: McpAccess) -> (Option<String>, Option<bool>) {
    if access != McpAccess::Network {
        return (None, None);
    }
    let extra = std::env::var("FFM_MCP_CONFIGS").ok();
    let tls = std::env::var("FFM_MCP_TLS_VERIFY").ok().and_then(|v| {
        match v.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "no" | "off" => Some(false),
            "1" | "true" | "yes" | "on" => Some(true),
            _ => None,
        }
    });
    (extra, tls)
}

#[cfg(test)]
#[must_use]
pub fn unique_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests;
