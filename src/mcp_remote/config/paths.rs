//! Purpose: Expand `${VAR}` MCP header templates and merge server discovery paths.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use super::{load_yaml_settings, parse_mcp_json, to_server};
use crate::values::mcp_remote::{HttpMcpServer, McpRemoteError};

/// Expand `${VAR}` references in a header template using `env`. Rejects unset or control values.
pub fn expand_template(
    template: &str,
    env: impl Fn(&str) -> Option<String>,
) -> Result<Zeroizing<String>, McpRemoteError> {
    let mut expanded = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, var_name, after)) = next_placeholder(rest)? {
        expanded.push_str(before);
        let secret = env(var_name)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| McpRemoteError::MissingEnv(var_name.to_owned()))?;
        if secret.chars().any(char::is_control) {
            return Err(McpRemoteError::Config(format!(
                "{var_name} must not contain control characters"
            )));
        }
        expanded.push_str(&secret);
        rest = after;
    }
    expanded.push_str(rest);
    if expanded.chars().any(char::is_control) {
        return Err(McpRemoteError::Config(
            "MCP header must not contain control characters".to_owned(),
        ));
    }
    Ok(Zeroizing::new(expanded))
}

fn next_placeholder(rest: &str) -> Result<Option<(&str, &str, &str)>, McpRemoteError> {
    let Some((before, tail)) = rest.split_once("${") else {
        return Ok(None);
    };
    let Some((var_name, remainder)) = tail.split_once('}') else {
        return Err(McpRemoteError::Config(
            "Unclosed ${VAR} placeholder in MCP header template".to_owned(),
        ));
    };
    let trimmed = var_name.trim();
    if trimmed.is_empty() {
        return Err(McpRemoteError::Config(
            "Empty ${} placeholder in MCP header".to_owned(),
        ));
    }
    Ok(Some((before, trimmed, remainder)))
}

/// Parse extra semicolon/colon-separated config file paths from an env string.
#[must_use]
pub fn split_config_paths(raw: Option<&str>) -> Vec<PathBuf> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    std::env::split_paths(raw)
        .filter(|path| !path.as_os_str().is_empty())
        .collect()
}

/// Merge HTTP MCP servers from candidate JSON files, extra env paths, and `ffm.yml`.
#[must_use]
pub fn discover(
    json_paths: &[PathBuf],
    yaml_path: Option<&Path>,
    extra_env_paths: Option<&str>,
    tls_env_override: Option<bool>,
) -> Vec<HttpMcpServer> {
    let yaml = yaml_path
        .and_then(|p| load_yaml_settings(p).ok())
        .unwrap_or_default();
    let default_tls = tls_env_override.or(yaml.tls_verify).unwrap_or(true);
    let default_ca = yaml.ca_cert.as_deref();
    let mut merged = BTreeMap::new();
    for path in json_paths
        .iter()
        .chain(yaml.configs.iter())
        .chain(split_config_paths(extra_env_paths).iter())
    {
        merge_json_file(path, default_tls, default_ca, &mut merged);
    }
    let yaml_label = yaml_path.map_or_else(|| "ffm.yml".to_owned(), |p| p.display().to_string());
    for (name, raw) in yaml.servers {
        if let Some(server) = to_server(name, raw, &yaml_label, default_tls, default_ca) {
            merged.insert(server.name.clone(), server);
        }
    }
    merged.into_values().collect()
}

fn merge_json_file(
    path: &Path,
    default_tls: bool,
    default_ca: Option<&Path>,
    merged: &mut BTreeMap<String, HttpMcpServer>,
) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let label = path.display().to_string();
    if let Ok(servers) = parse_mcp_json(&text, &label, default_tls, default_ca) {
        for server in servers {
            merged.insert(server.name.clone(), server);
        }
    }
}
