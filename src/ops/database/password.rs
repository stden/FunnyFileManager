//! Purpose: Where a PostgreSQL password comes from, and how driver text is kept
//! free of it. The value is read when a request connects and goes no further.

use super::DbError;
use crate::models::database::PostgresEndpoint;

const MASK: &str = "********";

/// The password the endpoint's variable holds, read through `lookup`; `None`
/// when the endpoint names no variable.
pub(super) fn resolve(
    endpoint: &PostgresEndpoint,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Option<String>, DbError> {
    let Some(variable) = &endpoint.password_env else {
        return Ok(None);
    };
    lookup(variable)
        .map(Some)
        .ok_or_else(|| DbError::MissingPassword(variable.clone()))
}

/// `text` with the secret taken out, so the text may be shown.
pub(super) fn redact(text: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() => text.replace(secret, MASK),
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests;
