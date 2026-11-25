//! Purpose: Plain values that every layer may use.
//!
//! A directory entry, an image, a remote path and a saved connection. None of
//! them depends on application state, so adapters may use them directly.

pub mod ai;
pub mod cloudflare_dns;
pub mod entry;
pub mod image;
pub mod jira;
pub mod mcp_remote;
pub mod remote_path;
pub mod session;

/// Return a trimmed non-empty string if `value` has visible characters.
#[must_use]
pub(crate) fn non_blank(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

/// Resolve an optional TLS verification boolean override from environment text or YAML config.
#[must_use]
pub(crate) fn parse_tls_verify(env_value: Option<String>, configured: Option<bool>) -> bool {
    if let Some(raw) = non_blank(env_value) {
        let lower = raw.to_ascii_lowercase();
        if matches!(lower.as_str(), "0" | "false" | "no" | "off") {
            return false;
        }
        if matches!(lower.as_str(), "1" | "true" | "yes" | "on") {
            return true;
        }
    }
    configured.unwrap_or(true)
}

#[cfg(test)]
mod tests;
