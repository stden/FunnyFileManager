//! Purpose: Bounded HTTPS reports; credentials never enter logs or persisted configuration.
use super::{Provider, UsageError, UsageHttp};
use chrono::{NaiveTime, Utc};
use reqwest::{
    blocking::Client as HttpClient,
    header::{HeaderMap, HeaderValue},
};
use std::{
    io::Read,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub(super) struct Client {
    inner: HttpClient,
    end: chrono::DateTime<Utc>,
    provider: Provider,
    key: ApiKey,
    deadline: Instant,
}
struct ApiKey(Zeroizing<String>);
impl Client {
    pub(super) fn new(provider: Provider) -> Result<Self, UsageError> {
        let key = std::env::var(provider.key_variable())
            .ok()
            .filter(|key| !key.trim().is_empty())
            .ok_or(UsageError::MissingKey)?;
        let key = ApiKey(key.into());
        let client = HttpClient::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_http_error| UsageError::Network)?;
        Ok(Self {
            inner: client,
            end: Utc::now().date_naive().and_time(NaiveTime::MIN).and_utc(),
            provider,
            key,
            deadline: Instant::now() + Duration::from_secs(45),
        })
    }
}
impl UsageHttp for Client {
    fn get(
        &mut self,
        provider: Provider,
        page: Option<&str>,
    ) -> Result<serde_json::Value, UsageError> {
        if provider != self.provider {
            return Err(UsageError::InvalidResponse);
        }
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(UsageError::Limit)?;
        let (url, query) = request(provider, page, self.end);
        let response = self
            .inner
            .get(url)
            .headers(headers(provider, &self.key)?)
            .query(&query)
            .timeout(remaining.min(Duration::from_secs(15)))
            .send()
            .map_err(|_http_error| UsageError::Network)?;
        read_response(response)
    }
}
fn headers(provider: Provider, key: &ApiKey) -> Result<HeaderMap, UsageError> {
    let mut headers = HeaderMap::new();
    let (name, value) = if provider == Provider::Anthropic {
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_static("analytics-usage-cost-2026-08-24"),
        );
        ("x-api-key", key.0.to_string())
    } else {
        ("authorization", format!("Bearer {}", key.0.as_str()))
    };
    let value = Zeroizing::new(value);
    let mut value =
        HeaderValue::from_str(&value).map_err(|_header_error| UsageError::AccessDenied)?;
    value.set_sensitive(true);
    headers.insert(name, value);
    Ok(headers)
}
fn read_response(response: reqwest::blocking::Response) -> Result<serde_json::Value, UsageError> {
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(UsageError::AccessDenied),
        429 => return Err(UsageError::RateLimited),
        _ => return Err(UsageError::Network),
    }
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_http_error| UsageError::Network)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(UsageError::Limit);
    }
    serde_json::from_slice(&bytes).map_err(|_json_error| UsageError::InvalidResponse)
}
fn request(
    provider: Provider,
    page: Option<&str>,
    end: chrono::DateTime<Utc>,
) -> (&'static str, Vec<(&'static str, String)>) {
    let start = end - chrono::Duration::days(30);
    let (url, mut query) = match provider {
        Provider::OpenAi => (
            "https://api.openai.com/v1/organization/usage/completions",
            vec![
                ("start_time", start.timestamp().to_string()),
                ("end_time", end.timestamp().to_string()),
                ("bucket_width", "1d".to_owned()),
                ("limit", "31".to_owned()),
            ],
        ),
        Provider::Anthropic => (
            "https://api.anthropic.com/v1/organizations/analytics/usage",
            vec![
                ("starting_at", start.to_rfc3339()),
                ("ending_at", end.to_rfc3339()),
                ("limit", "1000".to_owned()),
            ],
        ),
        Provider::OpenRouter => ("https://openrouter.ai/api/v1/activity", Vec::new()),
    };
    if let Some(page) = page {
        query.push(("page", page.to_owned()));
    }
    (url, query)
}

#[cfg(test)]
mod tests;
